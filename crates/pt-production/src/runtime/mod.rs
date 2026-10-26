//! Durable side effects. SQLite is the sole metadata/event/accounting authority.
#[allow(dead_code)] // Paid adapters await the shared quota/recovery boundary.
mod broker;
mod fixture;
mod helper;
mod media;
mod publication;
mod runner;
mod store;
#[cfg(test)]
use broker::{AccountingImport, Dispatch, Limits, Operation, Receipt, Transport};
pub use media::{
    MAX_ARTIFACT_BYTES, MAX_MASTER_BYTES, VerifiedArtifact, artifact_limit,
    validate_artifact_bytes, validate_artifact_size,
};
pub use runner::{FixtureExecutor, FixturePipeline, Runtime, Services};
pub use store::{Error, Result, STAGES, Store, digest, now};
#[cfg(test)]
mod acoustic_pipeline_tests;
#[cfg(test)]
mod master_wav_tests;
#[cfg(test)]
mod tests;
