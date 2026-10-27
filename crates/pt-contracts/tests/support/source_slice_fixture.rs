#[path = "acceptance_fixture.rs"]
pub mod acceptance_fixture;
use pt_contracts::*;
pub struct Fixture {
    pub f: acceptance_fixture::Fixture,
    pub policy: AcceptanceVerificationPolicyV2,
    pub relationship: SourceSliceRelationshipV1,
}
impl Fixture {
    pub fn new(prefix: &str, origin: u64) -> Self {
        let mut f = acceptance_fixture::Fixture::new();
        f.doc
            .acceptance
            .required_check_ids
            .push("complete-source-coverage".into());
        f.doc.acceptance.required_check_ids.sort();
        f.doc.acceptance.required_check_ids.dedup();
        f.save();
        let source = f
            .policy
            .inputs
            .iter()
            .find(|i| i.role == AcceptanceInputRole::Source)
            .unwrap()
            .file
            .clone();
        let full = format!("{prefix}{}\nFull source suffix.\u{301}", f.doc.source_text);
        let count = full.chars().count() as u64;
        let start = origin + prefix.chars().count() as u64;
        let n = f.doc.source_text.chars().count() as u64;
        let full_source = FullSourcePinV1 {
            file: acceptance_fixture::put(
                &f.private,
                "full-source.txt",
                full.as_bytes(),
                "text/plain",
            ),
            scope_id: "trusted-full-source".into(),
            scalar_count: count,
            origin,
        };
        let relationship = SourceSliceRelationshipV1 {
            schema_version: 1,
            full_source: full_source.clone(),
            slice: source,
            slice_scope_id: f.policy.source_scope_id.clone(),
            slice_scalar_count: n,
            full_source_span: [start, start + n],
            local_span: [0, n],
            local_origin: 0,
            global_origin: start,
        };
        let file = acceptance_fixture::put(
            &f.private,
            "source-relationship.json",
            &serde_json::to_vec(&relationship).unwrap(),
            "application/json",
        );
        let policy = AcceptanceVerificationPolicyV2 {
            schema_version: 2,
            base: f.policy.clone(),
            full_source,
            full_source_span: relationship.full_source_span,
            relationship: Some(file),
        };
        Self {
            f,
            policy,
            relationship,
        }
    }
    #[allow(dead_code)]
    pub fn save_relationship(&mut self) {
        self.policy.relationship = Some(acceptance_fixture::put(
            &self.f.private,
            "source-relationship.json",
            &serde_json::to_vec(&self.relationship).unwrap(),
            "application/json",
        ));
    }
    pub fn verify(&self) -> PrivateAcceptanceVerification {
        verify_acceptance_v2(
            &self.f.doc,
            &self.f.root,
            &self.policy,
            &[],
            &self.f.private,
        )
        .unwrap()
    }
}
pub fn source(r: &AcceptanceVerificationReport) -> &AcceptanceCheckResult {
    r.checks
        .iter()
        .find(|c| c.check_id == "complete-source-coverage")
        .unwrap()
}
