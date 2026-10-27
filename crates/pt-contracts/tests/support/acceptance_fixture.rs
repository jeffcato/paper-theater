use pt_contracts::*;
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
pub struct Fixture {
    pub root: PathBuf,
    pub private: PathBuf,
    pub doc: PlaybackDocument,
    pub policy: AcceptanceVerificationPolicy,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
pub fn put(root: &Path, path: &str, bytes: &[u8], media_type: &str) -> FileRef {
    let p = root.join(path);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, bytes).unwrap();
    FileRef {
        path: path.into(),
        sha256: digest(bytes),
        bytes: bytes.len() as u64,
        media_type: media_type.into(),
        dimensions: None,
    }
}
impl Fixture {
    pub fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "pt-acceptance-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&root).unwrap();
        let original = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/generation/two-scene-readiness-v2");
        fn copy(a: &Path, b: &Path) {
            std::fs::create_dir_all(b).unwrap();
            for e in std::fs::read_dir(a).unwrap() {
                let e = e.unwrap();
                if e.path().is_dir() {
                    copy(&e.path(), &b.join(e.file_name()))
                } else {
                    std::fs::copy(e.path(), b.join(e.file_name())).unwrap();
                }
            }
        }
        copy(&original, &root);
        let private = root.join("private-inputs");
        std::fs::create_dir(&private).unwrap();
        let mut doc: PlaybackDocument =
            serde_json::from_slice(&std::fs::read(root.join("fixture.json")).unwrap()).unwrap();
        let mut inputs = Vec::new();
        for (role, from, to, media) in [
            (
                AcceptanceInputRole::Source,
                "source.txt",
                "source.txt",
                "text/plain",
            ),
            (
                AcceptanceInputRole::Narration,
                "evidence/narration.wav",
                "narration.wav",
                "audio/wav",
            ),
            (
                AcceptanceInputRole::Alignment,
                "evidence/alignment.json",
                "alignment.json",
                "application/json",
            ),
            (
                AcceptanceInputRole::PresentationMap,
                "presentation-map.json",
                "map.json",
                "application/json",
            ),
            (
                AcceptanceInputRole::Profile,
                "profile.json",
                "profile.json",
                "application/json",
            ),
        ] {
            let b = std::fs::read(root.join(from)).unwrap();
            inputs.push(AcceptanceInput {
                role,
                file: put(&private, to, &b, media),
            });
        }
        let recipe = put(
            &private,
            "recipe.txt",
            b"zero-provider diagnostic acceptance binding recipe",
            "text/plain",
        );
        doc.acceptance.recipe_sha256 = recipe.sha256.clone();
        for a in &mut doc.acceptance.attestations {
            a.recipe_sha256 = recipe.sha256.clone();
        }
        inputs.push(AcceptanceInput {
            role: AcceptanceInputRole::Recipe,
            file: recipe,
        });
        let player = put(
            &private,
            "diagnostic-player.txt",
            b"synthetic player identity; not a production build",
            "text/plain",
        );
        inputs.push(AcceptanceInput {
            role: AcceptanceInputRole::Player,
            file: player,
        });
        let facts = doc
            .manifest
            .reveals
            .iter()
            .flat_map(|g| {
                g.facts.iter().enumerate().map(|(i, f)| AcceptanceFact {
                    id: format!("{}-fact-{i}", g.id),
                    reveal_id: g.id.clone(),
                    source_span: f.source_span,
                    asset_sha256: doc.actors[0].image.sha256.clone(),
                })
            })
            .collect();
        let source_span = [0, doc.source_text.chars().count() as u64];
        let inventory = AcceptanceFactInventory {
            schema_version: 1,
            source_scope_id: "pg-55-chapter-1-diagnostic".into(),
            source_sha256: doc.manifest.source_sha256.clone(),
            source_span,
            facts,
            required_cue_ids: doc.timeline.cues.iter().map(|c| c.id.clone()).collect(),
        };
        let fact_file = put(
            &private,
            "facts.json",
            &serde_json::to_vec(&inventory).unwrap(),
            "application/json",
        );
        inputs.push(AcceptanceInput {
            role: AcceptanceInputRole::FactInventory,
            file: fact_file,
        });
        doc.acceptance.required_check_ids.extend(
            required_acceptance_checks(AcceptanceScope::Diagnostic)
                .into_iter()
                .map(|c| c.id),
        );
        doc.acceptance.required_check_ids.sort();
        doc.acceptance.required_check_ids.dedup();
        doc.acceptance.player_version = "diagnostic-only".into();
        let mut assets = doc
            .manifest
            .assets
            .iter()
            .map(|f| f.sha256.clone())
            .collect::<Vec<_>>();
        assets.sort();
        assets.dedup();
        let ledger = put(
            &private,
            "evidence-inventory.json",
            br#"{"schema_version":1,"proposals":[]}"#,
            "application/json",
        );
        inputs.push(AcceptanceInput {
            role: AcceptanceInputRole::EvidenceInventory,
            file: ledger,
        });
        let policy = AcceptanceVerificationPolicy {
            schema_version: 1,
            scope: AcceptanceScope::Diagnostic,
            inputs,
            source_span,
            narration_sample_rate: 24000,
            narration_sample_count: 1451280,
            source_scope_id: "pg-55-chapter-1-diagnostic".into(),
            payload_inventory_sha256: doc.acceptance.inventory_sha256.clone(),
            asset_sha256s: assets,
            device_id: None,
            player_version: "diagnostic-only".into(),
            candidate_checker_version: doc.acceptance.checker_version.clone(),
            checker_sha256: acceptance_binding_checker().sha256,
            additional_required_check_ids: vec![],
        };
        let mut f = Self {
            root,
            private,
            doc,
            policy,
        };
        f.save();
        f
    }
    pub fn save(&mut self) {
        let b = serde_json::to_vec(&self.doc.acceptance).unwrap();
        self.doc.manifest.acceptance = put(&self.root, "acceptance.json", &b, "application/json");
        put(
            &self.root,
            "manifest.json",
            &serde_json::to_vec(&self.doc.manifest).unwrap(),
            "application/json",
        );
    }
}
