//! Durable side effects. SQLite is the sole metadata/event/accounting authority.
mod acceptance;
#[allow(dead_code)] // Paid adapters await the shared quota/recovery boundary.
mod broker;
mod chapter;
mod fixture;
mod helper;
mod media;
mod presentation;
mod provider_service;
#[allow(dead_code)] // Persistent broker; real construction/transport remain disabled.
mod provider_v2;
mod publication;
mod runner;
mod store;
#[cfg(test)]
use broker::{AccountingImport, Dispatch, Limits, Operation, Receipt, Transport};
pub use chapter::{ChapterHelper, ChapterUnitState};
pub use media::{
    MAX_ARTIFACT_BYTES, MAX_MASTER_BYTES, VerifiedArtifact, artifact_limit,
    validate_artifact_bytes, validate_artifact_size,
};
pub use runner::{FixtureExecutor, FixturePipeline, Runtime, Services};
pub use store::{Error, Result, STAGES, Store, digest, now};
#[cfg(test)]
mod acceptance_tests;
#[cfg(test)]
mod acoustic_pipeline_tests;
#[cfg(test)]
mod chapter_tests;
#[cfg(test)]
mod master_wav_tests;
#[cfg(test)]
mod presentation_pipeline_tests;
#[cfg(test)]
mod provider_v2_tests;
#[cfg(test)]
mod tests;
