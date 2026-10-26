use pt_contracts::*;
#[test]
fn committed_events_roundtrip_with_causal_fields_and_reject_unknown_kinds() {
    let event=RunEvent{schema_version:1,run_id:"run-1".into(),seq:39,epoch:2,revision:3,timestamp:"2026-10-26T17:00:00Z".into(),entry:JournalProposal{kind:EventKind::ArtifactReady,task_id:Some("package".into()),decision_id:None,artifact_ids:vec!["artifact-1".into()],summary:"Diagnostic artifact stored".into()}};
    let mut wire=serde_json::to_value(&event).unwrap();
    let read:RunEvent=serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(read.seq,39);assert_eq!(read.entry.artifact_ids,["artifact-1"]);
    wire["kind"]="unrecognized".into();assert!(serde_json::from_value::<RunEvent>(wire).is_err());
}
