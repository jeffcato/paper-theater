//! Synthetic PCM exercises the reconstruction API without altering any provider observation.
use crate::generation::presentation_pcm::verify;
use pt_contracts::digest;
use serde_json::{Value, json};
fn wav(rate: u32, channels: u16, samples: &[i16]) -> Vec<u8> {
    let data = samples
        .iter()
        .flat_map(|s| s.to_le_bytes())
        .collect::<Vec<_>>();
    let mut b = Vec::new();
    b.extend(b"RIFF");
    b.extend((36 + data.len() as u32).to_le_bytes());
    b.extend(b"WAVEfmt ");
    b.extend(16u32.to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend(channels.to_le_bytes());
    b.extend(rate.to_le_bytes());
    b.extend((rate * channels as u32 * 2).to_le_bytes());
    b.extend((channels * 2).to_le_bytes());
    b.extend(16u16.to_le_bytes());
    b.extend(b"data");
    b.extend((data.len() as u32).to_le_bytes());
    b.extend(data);
    b
}
fn edit(id: &str, delay: u64, compensation: u64, trim_start: u64, trim_end: u64) -> Value {
    json!({"id":id,"processing_delay_frames":delay,"compensation_frames":compensation,"trim_start_frames":trim_start,"trim_end_frames":trim_end})
}
fn bundle(
    segments: Value,
    gaps: Value,
    edits: Value,
    omissions: Value,
    mono: &[i16],
) -> [Vec<u8>; 4] {
    let source = wav(32000, 1, &[100, 200, 300, 400]);
    let stereo = mono.iter().flat_map(|s| [*s, *s]).collect::<Vec<_>>();
    let delivered = wav(48000, 2, &stereo);
    let map = json!({"schema_version":1,"source_audio_sha256":digest(&source),"source_sample_rate":32000,"delivery_sample_rate":48000,"segments":segments,"gaps":gaps});
    let map = serde_json::to_vec(&map).unwrap();
    let recipe = json!({"schema_version":1,"provenance":"diagnostic","kind":"pcm-reconstruction-recipe; not an original execution receipt","algorithm":"rational-zero-order-hold-pcm16-v1","channel_map":"mono-duplicate-stereo",
        "narration_sha256":digest(&source),"delivery_sha256":digest(&delivered),"map_sha256":digest(&map),"source_frames":4,"delivery_frames":mono.len(),"source_omissions":omissions,"segments":edits,"historical":null});
    [map, serde_json::to_vec(&recipe).unwrap(), source, delivered]
}
#[test]
fn inserted_gaps_cut_end_ownership_cross_segment_coverage_and_fractional_ceiling() {
    let [m, r, s, d] = bundle(
        json!([{"source":[0,2],"destination":[1,4]},{"source":[2,4],"destination":[7,10]}]),
        json!([[0, 1], [4, 7], [10, 12]]),
        json!([edit("a", 0, 0, 0, 0), edit("b", 0, 0, 0, 0)]),
        json!([]),
        &[0, 100, 100, 200, 0, 0, 0, 300, 300, 400, 0, 0],
    );
    let verified = verify(&m, &r, &s, &d).unwrap();
    assert_eq!(verified.interval([0, 1]).unwrap().0, 3); // 1 + ceil(1 * 48000 / 32000)
    assert_eq!(verified.interval([0, 2]).unwrap().0, 4); // exact cut ends before the inserted gap
    assert_eq!(verified.point(2, false).unwrap(), 7); // a start at that cut belongs after the gap
    assert_eq!(verified.point(2, true).unwrap(), 4);
    let (bound, pieces) = verified.interval([1, 3]).unwrap();
    assert_eq!(bound, 9);
    assert_eq!(pieces.len(), 2);
    assert_eq!(verified.interval([2, 4]).unwrap().0, 10); // trailing silence never moves the bound
}
#[test]
fn verified_trims_and_delay_compensation_keep_only_complete_intervals() {
    let [m, r, s, d] = bundle(
        json!([{"source":[0,4],"destination":[2,6]}]),
        json!([[0, 2], [6, 8]]),
        json!([edit("trim", 4, 2, 3, 1)]),
        json!([]),
        &[0, 0, 100, 200, 300, 300, 0, 0],
    );
    let verified = verify(&m, &r, &s, &d).unwrap();
    assert_eq!(verified.interval([1, 3]).unwrap().0, 6);
    assert_eq!(
        verified.interval([0, 1]).err().unwrap().reason,
        "interval_trimmed"
    );
    assert_eq!(
        verified.interval([2, 4]).err().unwrap().reason,
        "interval_trimmed"
    );
    for (compensation, pcm, bound) in [
        (0, vec![0, 0, 100, 100, 200, 300, 300, 400], 4),
        (2, vec![100, 100, 200, 300, 300, 400], 2),
    ] {
        let n = pcm.len();
        let [m, r, s, d] = bundle(
            json!([{"source":[0,4],"destination":[0,n]}]),
            json!([]),
            json!([edit("delay", 2, compensation, 0, 0)]),
            json!([]),
            &pcm,
        );
        assert_eq!(
            verify(&m, &r, &s, &d).unwrap().interval([0, 1]).unwrap().0,
            bound
        );
    }
}
#[test]
fn omitted_or_reordered_speech_and_invalid_domains_cannot_get_a_bound() {
    let parts = json!([{"source":[0,1],"destination":[0,2]},{"source":[2,4],"destination":[2,5]}]);
    let edits = json!([edit("a", 0, 0, 0, 0), edit("b", 0, 0, 0, 0)]);
    let [m, r, s, d] = bundle(
        parts.clone(),
        json!([]),
        edits.clone(),
        json!([[1, 2]]),
        &[100, 100, 300, 300, 400],
    );
    let verified = verify(&m, &r, &s, &d).unwrap();
    assert_eq!(
        verified.interval([0, 3]).err().unwrap().reason,
        "interval_unmapped"
    );
    assert_eq!(verified.interval([2, 4]).unwrap().0, 5);
    assert!(verified.interval([3, 5]).is_err());
    assert!(verified.interval([2, 2]).is_err());
    for fault in [
        "omission",
        "reorder",
        "overlap",
        "source-range",
        "destination-range",
        "undeclared-gap",
        "rate",
        "huge-edit",
        "compensation",
        "recipe",
        "pcm",
    ] {
        let [mut m, mut r, s, mut d] = bundle(
            parts.clone(),
            json!([]),
            edits.clone(),
            json!([[1, 2]]),
            &[100, 100, 300, 300, 400],
        );
        let mut map: Value = serde_json::from_slice(&m).unwrap();
        let mut recipe: Value = serde_json::from_slice(&r).unwrap();
        match fault {
            "omission" => recipe["source_omissions"] = json!([]),
            "reorder" => map["segments"].as_array_mut().unwrap().reverse(),
            "overlap" => map["segments"][1]["source"] = json!([0, 2]),
            "source-range" => map["segments"][1]["source"] = json!([2, 5]),
            "destination-range" => map["segments"][1]["destination"] = json!([2, 6]),
            "undeclared-gap" => map["segments"][1]["destination"] = json!([3, 5]),
            "rate" => map["source_sample_rate"] = json!(0),
            "huge-edit" => {
                recipe["segments"][0]["processing_delay_frames"] = json!(9007199254740992u64)
            }
            "compensation" => recipe["segments"][0]["compensation_frames"] = json!(1),
            "recipe" => recipe["algorithm"] = json!("unknown"),
            "pcm" => d[44] ^= 1,
            _ => unreachable!(),
        }
        m = serde_json::to_vec(&map).unwrap();
        recipe["map_sha256"] = json!(digest(&m));
        recipe["delivery_sha256"] = json!(digest(&d));
        r = serde_json::to_vec(&recipe).unwrap();
        assert!(verify(&m, &r, &s, &d).is_err(), "accepted {fault}");
    }
}
