//! Proposed PCM-copy domains only. No audio assembly, expected prose, transcription or timestamps.
use super::*;
struct Batch {
    rate: u64,
    channels: u64,
    frames: u64,
    segments: Vec<Value>,
}
impl Batch {
    fn value(self, index: usize) -> Value {
        json!({"id":format!("b{index:04}"),"sample_rate":self.rate,"channels":self.channels,"bits_per_sample":16,"frames":self.frames,"wav_bytes_if_materialized":44+self.frames*self.channels*2,"duration":{"numerator":self.frames,"denominator":self.rate},"segments":self.segments,"sample_origin":"proposed-batch-local","materialized_audio":null,"proposal":null,"model_payload":{"schema_version":1,"mode":"VERBATIM","word_timestamps":true,"audio":null}})
    }
}
pub(super) fn plan(
    takes: &[Value],
    ctx: &Context,
    findings: &mut Vec<Finding>,
) -> Result<Value, StageError> {
    let mut batches = vec![];
    let mut current: Option<Batch> = None;
    let mut previous_end = None;
    let mut cut = false;
    let mut unplanned = vec![];
    for take in takes {
        let rate = take["native"]["sample_rate"].as_u64().unwrap();
        let channels = take["native"]["channels"].as_u64().unwrap();
        let frames = take["native"]["frames"].as_u64().unwrap();
        let cap = (ctx.limits.transcription_max_wav_bytes - 44) / (channels * 2);
        let cap = cap.min(rate * ctx.limits.transcription_max_seconds as u64);
        if cap == 0 {
            unplanned.push(
                json!({"unit_id":take["unit_id"],"audio":take["audio"],"native_span":[0,frames]}),
            );
            findings.push(missing("speech-transcription-native-limit","Configured WAV budget cannot hold a native frame; original recording remains unmodified and unplanned."));
            continue;
        }
        if current.as_ref().is_some_and(|b| {
            b.rate != rate
                || b.channels != channels
                || previous_end != take["source_span"][0].as_u64()
        }) {
            batches.push(current.take().unwrap().value(batches.len()));
        }
        let mut a = 0;
        while a < frames {
            if current.as_ref().is_some_and(|b| {
                b.frames == cap
                    || b.segments.len() >= ctx.limits.transcription_max_segments as usize
            }) {
                batches.push(current.take().unwrap().value(batches.len()));
            }
            if batches.len() >= ctx.limits.transcription_max_batches as usize {
                unplanned.push(json!({"unit_id":take["unit_id"],"audio":take["audio"],"native_span":[a,frames]}));
                findings.push(missing("speech-transcription-plan-limit","Bounded request fanout reached; remaining original native frame ranges stay explicit and unresolved."));
                break;
            }
            let b = current.get_or_insert_with(|| Batch {
                rate,
                channels,
                frames: 0,
                segments: vec![],
            });
            let end = (a + cap - b.frames).min(frames);
            let dest = b.frames;
            let len = end - a;
            need(len > 0, "transcription plan made no frame progress")?;
            let crossed = a > 0 || end < frames;
            cut |= crossed;
            b.segments.push(json!({"unit_id":take["unit_id"],"audio":take["audio"],"native_span":[a,end],"batch_span":[dest,dest+len],"cross_cut_uncertainty":crossed}));
            b.frames += len;
            a = end;
        }
        previous_end = take["source_span"][1].as_u64();
    }
    if let Some(b) = current {
        batches.push(b.value(batches.len()));
    }
    if cut {
        findings.push(missing("speech-transcription-cross-cut","PCM-copy plan crosses native take interiors; future measured interval/fact mappings must preserve boundary uncertainty."));
    }
    Ok(
        json!({"schema_version":1,"provenance":"diagnostic","provider":ctx.transcription_provider,"model":ctx.transcription_model,"recipe":ctx.recipe,"copy_recipe":"native-pcm16-concatenation-plan-v1","copy_recipe_code_sha256":digest(include_bytes!("speech_transcription.rs")),"batch_materialization_required":true,"batches":batches,"unplanned_audio":unplanned,"expected_prose_supplied":false,"source_matching":"local-only-after-measured-responses","measured_word_intervals":null,"fact_bounds":null,"narration_assembly":null,"dispatch_permitted":false,"production_approved":false}),
    )
}
