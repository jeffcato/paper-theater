use super::*;
fn wav(values: &[i16], channels: u16, rate: u32, junk: bool) -> Vec<u8> {
    let frames = values.len() as u64 / channels as u64;
    let mut b = header(&batch("b0000", rate, channels, frames, vec![])).unwrap();
    if junk {
        b.extend_from_slice(b"JUNK");
        b.extend_from_slice(&4u32.to_le_bytes());
        b.extend_from_slice(&[99; 4]);
    }
    for x in values {
        b.extend_from_slice(&x.to_le_bytes());
    }
    if junk {
        let data = b.split_off(44);
        let mut out = b[..36].to_vec();
        out.extend_from_slice(&data[..12]);
        out.extend_from_slice(&b[36..44]);
        out.extend_from_slice(&data[12..]);
        b = out;
    }
    let len = b.len() as u32 - 8;
    b[4..8].copy_from_slice(&len.to_le_bytes());
    b
}
fn limits() -> Limits {
    Limits {
        transcription_max_wav_bytes: 16 * META,
        transcription_max_seconds: 90,
        transcription_max_segments: 64,
        transcription_max_batches: 256,
        max_unit_text_bytes: 32,
        max_units: 256,
    }
}
fn batch(id: &str, rate: u32, channels: u16, frames: u64, segments: Vec<Segment>) -> Batch {
    Batch {
        id: id.into(),
        sample_rate: rate,
        channels,
        bits_per_sample: 16,
        frames,
        wav_bytes_if_materialized: 44 + frames * channels as u64 * 2,
        duration: Duration {
            numerator: frames,
            denominator: rate as u64,
        },
        segments,
        sample_origin: "proposed-batch-local".into(),
        materialized_audio: None,
        proposal: None,
        model_payload: json!({"schema_version":1,"mode":"VERBATIM","word_timestamps":true,"audio":null}),
    }
}
fn audio(id: &str, b: &[u8]) -> c::ArtifactRef {
    artifact(
        "pcm-test",
        id,
        file(&format!("private/speech-audio/{id}.wav"), b, "audio/wav"),
    )
}
fn segment(id: &str, a: &c::ArtifactRef, native: [u64; 2], dest: [u64; 2], cut: bool) -> Segment {
    Segment {
        unit_id: id.into(),
        audio: a.clone(),
        native_span: native,
        batch_span: dest,
        cross_cut_uncertainty: cut,
    }
}
#[test]
fn malformed_batch_offsets_fail_without_overflow_or_partial_output() {
    let bytes = wav(&[101, -202], 1, 24000, false);
    let a = audio("a", &bytes);
    let s = segment("a", &a, [0, 2], [u64::MAX / 2, u64::MAX / 2 + 2], false);
    let b = batch("b0000", 24000, 1, 2, vec![s]);
    assert!(copy(&b, &limits(), |_| Ok(bytes.clone()), || Ok(())).is_err());
}
fn fixture() -> (Value, Value, Limits, BTreeMap<String, Vec<u8>>) {
    let one = wav(&[101, 202, 303, 404, 505, 606, 707, 808], 1, 24000, true);
    let two = wav(&[-11, -22, -33, -44, -55, -66], 1, 24000, false);
    let a = audio("a", &one);
    let b = audio("b", &two);
    let takes = json!({"schema_version":1,"production_approved":false,"narration_assembly":null,"delivery_master":null,"source_sha256":digest(b"source"),"takes":[{"unit_id":"a","audio":a,"source_span":[0,4],"source_sha256":digest(b"source"),"native_span":[0,8],"native":{"sample_rate":24000,"channels":1,"bits_per_sample":16,"frames":8},"global_narration_span":null,"production_approved":false},{"unit_id":"b","audio":b,"source_span":[4,8],"source_sha256":digest(b"source"),"native_span":[0,6],"native":{"sample_rate":24000,"channels":1,"bits_per_sample":16,"frames":6},"global_narration_span":null,"production_approved":false}]});
    let batches = vec![
        batch(
            "b0000",
            24000,
            1,
            3,
            vec![segment("a", &a, [0, 3], [0, 3], true)],
        ),
        batch(
            "b0001",
            24000,
            1,
            7,
            vec![
                segment("a", &a, [3, 8], [0, 5], true),
                segment("b", &b, [0, 2], [5, 7], true),
            ],
        ),
        batch(
            "b0002",
            24000,
            1,
            4,
            vec![segment("b", &b, [2, 6], [0, 4], true)],
        ),
    ];
    let plan = json!({"schema_version":1,"copy_recipe":"native-pcm16-concatenation-plan-v1","copy_recipe_code_sha256":digest(include_bytes!("../../generation/speech_transcription.rs")),"expected_prose_supplied":false,"dispatch_permitted":false,"production_approved":false,"batch_materialization_required":true,"measured_word_intervals":null,"fact_bounds":null,"narration_assembly":null,"batches":batches,"unplanned_audio":[]});
    (
        plan,
        takes,
        limits(),
        BTreeMap::from([(a.id, one), (b.id, two)]),
    )
}
#[test]
fn distinct_native_subranges_use_actual_data_chunks_and_preserve_frame_mapping() {
    let (plan, takes, limits, bytes) = fixture();
    let batches = validate(
        &plan,
        &takes,
        &limits,
        |a| Ok(bytes[&a.id].clone()),
        || Ok(()),
    )
    .unwrap();
    let expected = [
        vec![101, 202, 303],
        vec![404, 505, 606, 707, 808, -11, -22],
        vec![-33, -44, -55, -66],
    ];
    for (b, expected) in batches.iter().zip(expected) {
        let out = copy(b, &limits, |a| Ok(bytes[&a.id].clone()), || Ok(())).unwrap();
        let format = c::inspect_pcm16_wav(&mut Cursor::new(&out), out.len() as u64).unwrap();
        assert_eq!(format.data_offset, 44);
        let samples: Vec<i16> = out[44..]
            .chunks_exact(2)
            .map(|s| i16::from_le_bytes([s[0], s[1]]))
            .collect();
        assert_eq!(samples, expected);
        assert!(b.segments.iter().all(|s| s.cross_cut_uncertainty));
    }
}
#[test]
fn plan_rejects_missing_repeated_swapped_mixed_frames_hints_and_every_configured_cap() {
    let (plan, takes, base, bytes) = fixture();
    let check = |p: &Value, t: &Value, l: &Limits| {
        validate(p, t, l, |a| Ok(bytes[&a.id].clone()), || Ok(())).is_err()
    };
    for kind in 0..12 {
        let mut p = plan.clone();
        let mut t = takes.clone();
        let mut l = base.clone();
        match kind {
            0 => p["batches"][0]["segments"][0]["native_span"] = json!([1, 4]),
            1 => p["batches"][1]["segments"][0]["native_span"] = json!([0, 5]),
            2 => p["batches"].as_array_mut().unwrap().swap(0, 1),
            3 => t["takes"][1]["native"]["channels"] = json!(2),
            4 => t["takes"][1]["native"]["frames"] = json!(7),
            5 => p["batches"][0]["segments"][0]["cross_cut_uncertainty"] = json!(false),
            6 => p["batches"][0]["model_payload"]["expected_text"] = json!("forged hint"),
            7 => l.transcription_max_wav_bytes = 49,
            8 => l.transcription_max_segments = 1,
            9 => l.transcription_max_batches = 2,
            10 => p["batches"][0]["frames"] = json!(u64::MAX),
            _ => {
                t["takes"][1]["source_span"] = json!([5, 9]);
            }
        }
        assert!(check(&p, &t, &l), "accepted invalid case {kind}");
    }
    let mut l = base.clone();
    l.transcription_max_seconds = 1;
    let mut p = plan.clone();
    p["batches"][0]["frames"] = json!(24001);
    p["batches"][0]["wav_bytes_if_materialized"] = json!(48046);
    p["batches"][0]["duration"]["numerator"] = json!(24001);
    p["batches"][0]["segments"][0]["batch_span"] = json!([0, 24001]);
    assert!(check(&p, &takes, &l));
    let mut many = plan.clone();
    many["batches"] = json!(vec![plan["batches"][0].clone(); 257]);
    assert!(check(&many, &takes, &base));
}
#[test]
fn cancellation_inside_chunk_copy_returns_no_audio() {
    let values: Vec<i16> = (0..48000).map(|i| (i % 32767) as i16).collect();
    let bytes = wav(&values, 1, 24000, false);
    let a = audio("cancel", &bytes);
    let b = batch(
        "b0000",
        24000,
        1,
        48000,
        vec![segment("cancel", &a, [0, 48000], [0, 48000], false)],
    );
    let mut checks = 0;
    let result = copy(
        &b,
        &limits(),
        |_| Ok(bytes.clone()),
        || {
            checks += 1;
            if checks >= 5 {
                return fail("stale_epoch", "cancelled during PCM copy");
            }
            Ok(())
        },
    );
    assert_eq!(result.unwrap_err().code, "stale_epoch");
    assert_eq!(checks, 5);
}
