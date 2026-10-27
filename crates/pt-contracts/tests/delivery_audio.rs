#[path = "support/acceptance_fixture.rs"]
mod fixture;
use pt_contracts::*;
#[test]
fn saved_hash_verified_delivery_audio_is_independently_checked() {
    let mut f = fixture::Fixture::new();
    f.doc
        .acceptance
        .required_check_ids
        .push("audio-format-review".into());
    f.save();
    let v = verify_acceptance(&f.doc, &f.root, &f.policy, &[], &f.private).unwrap();
    let r = v.report();
    let c = r
        .checks
        .iter()
        .find(|c| c.check_id == "audio-format-review")
        .unwrap();
    assert!(c.executed, "actual format checker did not execute: {c:?}");
    assert_eq!(c.outcome, ReviewOutcome::Pass);
    assert!(
        c.consumed_sha256s
            .contains(&f.doc.manifest.entries.audio.sha256)
    );
    let measured = r.delivery_audio.as_ref().unwrap();
    assert_eq!(measured.format.channels, 2);
    assert_eq!(measured.format.sample_rate, 48000);
    assert_eq!(measured.format.bits_per_sample, 16);
    assert_eq!(measured.format.frame_count, f.doc.manifest.sample_count);
    assert_eq!(measured.format.frame_count, f.doc.timeline.sample_count);
    assert_eq!(measured.master_sha256, f.doc.manifest.entries.audio.sha256);
    assert_eq!(
        c.target,
        AcceptanceTarget::Audio {
            domain: AcceptanceSampleDomain::Delivery,
            span: [0, f.doc.manifest.sample_count]
        }
    );
    r.validate(&acceptance_binding_checker()).unwrap();
    assert!(!r.production_approved);
    assert!(
        r.blockers
            .iter()
            .any(|s| s == "production-verifier-pending")
    );
}

#[test]
fn physical_frames_must_match_both_manifest_and_timeline() {
    for timeline in [false, true] {
        let mut f = fixture::Fixture::new();
        f.doc
            .acceptance
            .required_check_ids
            .push("audio-format-review".into());
        if timeline {
            f.doc.timeline.sample_count += 1;
            f.doc.manifest.entries.timeline = fixture::put(
                &f.root,
                &f.doc.manifest.entries.timeline.path,
                &serde_json::to_vec(&f.doc.timeline).unwrap(),
                "application/json",
            );
        } else {
            f.doc.manifest.sample_count += 1;
        }
        f.save();
        let v = verify_acceptance(&f.doc, &f.root, &f.policy, &[], &f.private).unwrap();
        let c = v
            .report()
            .checks
            .iter()
            .find(|c| c.check_id == "audio-format-review")
            .unwrap();
        assert_eq!(
            c.outcome,
            ReviewOutcome::Fail,
            "physical frame mismatch was not diagnosed: {c:?}"
        );
        assert!(
            c.reasons
                .contains(&AcceptanceReason::AudioFrameCountMismatch)
        );
    }
}

#[path = "support/audio_fixture.rs"]
mod audio_fixture;
#[path = "support/wav_cases.rs"]
mod wav;
#[test]
fn malformed_and_wrong_domain_media_fail_even_when_hash_and_mime_match() {
    for case in wav::cases() {
        let f = audio_fixture::with_audio(&case.bytes, 1);
        let v = audio_fixture::verify(&f);
        let c = audio_fixture::audio(v.report());
        assert!(c.executed);
        assert_eq!(
            c.outcome == ReviewOutcome::Pass,
            case.delivery,
            "{}: {c:?}",
            case.id
        );
        if !case.native {
            assert!(
                c.reasons.contains(&AcceptanceReason::InvalidAudioFormat),
                "{}",
                case.id
            );
        } else if !case.delivery {
            assert!(
                c.reasons.contains(&AcceptanceReason::AudioDomainMismatch),
                "{}",
                case.id
            );
        }
        if let Some(a) = &v.report().delivery_audio {
            assert_eq!(a.format.frame_count, 1);
            assert_eq!(a.master_sha256, f.doc.manifest.entries.audio.sha256);
        }
        v.report().validate(&acceptance_binding_checker()).unwrap();
        assert!(!v.report().production_approved);
    }
}
#[test]
fn missing_unsafe_mislabeled_swapped_and_mutated_master_are_not_accepted() {
    for case in 0..5 {
        let mut f = audio_fixture::saved();
        match case {
            0 => {
                std::fs::remove_file(f.root.join(&f.doc.manifest.entries.audio.path)).unwrap();
            }
            1 => {
                f.doc.manifest.entries.audio.path = "../escape.wav".into();
                f.save();
            }
            2 => {
                f.doc.manifest.entries.audio.media_type = "application/octet-stream".into();
                f.save();
            }
            3 => {
                f.doc.manifest.entries.audio.sha256 = "0".repeat(64);
                f.save();
            }
            _ => {
                let native = f
                    .policy
                    .inputs
                    .iter()
                    .find(|i| i.role == AcceptanceInputRole::Narration)
                    .unwrap();
                f.doc.manifest.entries.audio = fixture::put(
                    &f.root,
                    "swapped.wav",
                    &std::fs::read(f.private.join(&native.file.path)).unwrap(),
                    "audio/wav",
                );
                f.save();
            }
        }
        let v = audio_fixture::verify(&f);
        let c = audio_fixture::audio(v.report());
        assert_ne!(c.outcome, ReviewOutcome::Pass, "case {case}");
        if case == 0 {
            assert_eq!(c.outcome, ReviewOutcome::Missing);
        }
    }
    let f = audio_fixture::saved();
    let v = audio_fixture::verify(&f);
    assert_eq!(
        audio_fixture::audio(v.report()).outcome,
        ReviewOutcome::Pass
    );
    let path = f.root.join(&f.doc.manifest.entries.audio.path);
    use std::io::{Read, Seek, SeekFrom, Write};
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();
    file.seek(SeekFrom::End(-1)).unwrap();
    let mut byte = [0];
    file.read_exact(&mut byte).unwrap();
    file.seek(SeekFrom::End(-1)).unwrap();
    file.write_all(&[byte[0] ^ 1]).unwrap();
    assert!(v.recheck_inputs().is_err());
    assert_ne!(
        audio_fixture::audio(audio_fixture::verify(&f).report()).outcome,
        ReviewOutcome::Pass
    );
}
#[test]
fn valid_format_never_erases_adverse_findings_or_executes_quality_reviews() {
    let mut f = audio_fixture::saved();
    f.doc.acceptance.findings.push(Finding {
        id: "audio-format-review".into(),
        field: "audio".into(),
        outcome: ReviewOutcome::Uncertain,
        message: "retained observation".into(),
        evidence: vec![],
    });
    f.save();
    let v = audio_fixture::verify(&f);
    let c = audio_fixture::audio(v.report());
    assert!(c.executed && c.outcome != ReviewOutcome::Pass);
    assert!(c.reasons.contains(&AcceptanceReason::AdverseEvidence));
    assert!(
        v.report()
            .checks
            .iter()
            .filter(|c| [
                "speech-fidelity",
                "mix-intelligibility",
                "narrator-continuity"
            ]
            .contains(&c.check_id.as_str()))
            .all(|c| !c.executed && c.outcome != ReviewOutcome::Pass)
    );
    assert!(!v.report().production_approved);
}

#[test]
fn diagnostic_imported_synthetic_and_forged_autonomous_labels_never_approve_audio() {
    for (prov, scope) in [
        (Provenance::Diagnostic, AcceptanceScope::Diagnostic),
        (Provenance::Imported, AcceptanceScope::AutonomousExcerpt),
        (Provenance::Synthetic, AcceptanceScope::ProductionChapter),
        (Provenance::Autonomous, AcceptanceScope::ProductionChapter),
    ] {
        let mut f = audio_fixture::with_audio(&wav::wave(2, 48000, 1), 1);
        f.policy.scope = scope;
        f.doc.manifest.provenance = prov;
        f.doc.acceptance.provenance = prov;
        f.save();
        let v = audio_fixture::verify(&f);
        assert_eq!(
            audio_fixture::audio(v.report()).outcome,
            ReviewOutcome::Pass
        );
        assert!(!v.report().production_approved);
        assert!(
            v.report()
                .blockers
                .iter()
                .any(|s| s == "production-verifier-pending")
        );
    }
    let mut f = audio_fixture::with_audio(&vec![b'X'; 48], 1);
    f.doc.manifest.provenance = Provenance::Autonomous;
    f.doc.acceptance.provenance = Provenance::Autonomous;
    f.doc.shell.profile.status = ProfileStatus::Verified;
    f.doc.acceptance.findings.push(Finding {
        id: "audio-format-review".into(),
        field: "audio".into(),
        outcome: ReviewOutcome::Pass,
        message: "untrusted claimed pass".into(),
        evidence: vec![],
    });
    f.save();
    let v = audio_fixture::verify(&f);
    assert_eq!(
        audio_fixture::audio(v.report()).outcome,
        ReviewOutcome::Fail
    );
    assert!(v.report().delivery_audio.is_none());
    assert!(!v.report().production_approved);
}

#[test]
fn unsupported_playback_schema_cannot_supply_a_delivery_count_domain() {
    let mut f = audio_fixture::saved();
    f.doc.timeline.schema_version = 2;
    f.doc.manifest.entries.timeline = fixture::put(
        &f.root,
        &f.doc.manifest.entries.timeline.path,
        &serde_json::to_vec(&f.doc.timeline).unwrap(),
        "application/json",
    );
    f.save();
    let v = audio_fixture::verify(&f);
    assert_ne!(
        audio_fixture::audio(v.report()).outcome,
        ReviewOutcome::Pass
    );
}
