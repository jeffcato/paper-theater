use super::fixture;
use pt_contracts::*;
pub fn saved() -> fixture::Fixture {
    let mut f = fixture::Fixture::new();
    f.doc
        .acceptance
        .required_check_ids
        .push("audio-format-review".into());
    f.save();
    f
}
pub fn with_audio(bytes: &[u8], frames: u64) -> fixture::Fixture {
    let mut f = saved();
    f.doc.manifest.entries.audio = fixture::put(
        &f.root,
        &f.doc.manifest.entries.audio.path,
        bytes,
        "audio/wav",
    );
    f.doc.manifest.sample_count = frames;
    f.doc.timeline.sample_count = frames;
    f.doc.manifest.entries.timeline = fixture::put(
        &f.root,
        &f.doc.manifest.entries.timeline.path,
        &serde_json::to_vec(&f.doc.timeline).unwrap(),
        "application/json",
    );
    f.save();
    f
}
pub fn verify(f: &fixture::Fixture) -> PrivateAcceptanceVerification {
    verify_acceptance(&f.doc, &f.root, &f.policy, &[], &f.private).unwrap()
}
pub fn audio(r: &AcceptanceVerificationReport) -> &AcceptanceCheckResult {
    r.checks
        .iter()
        .find(|c| c.check_id == "audio-format-review")
        .unwrap()
}
