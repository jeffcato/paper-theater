//! Shared wire records. Validation never grants production acceptance.
pub mod common;
pub mod services;
pub use common::*;
pub use services::*;
pub mod provider_metadata;
pub use provider_metadata::*;

pub const CONTRACT_REVISION: &str = "pt-contracts-v1.0.0";
pub const SERVICE_REVISION: &str = "pt-services-v1.10.0";
pub const CAPABILITY: &str = "paper-player-v1";

pub mod playback;
pub mod validate;
pub use playback::*;
pub use validate::*;
pub mod acceptance;
pub use acceptance::*;

pub mod provider_service;
pub use provider_service::*;

mod acceptance_source;

pub mod source_slice;
pub use source_slice::*;

pub mod wav;
pub use wav::*;

pub mod pilot_review;
pub use pilot_review::*;

pub mod production_configuration;
pub use production_configuration::*;

pub mod acceptance_coverage;
pub use acceptance_coverage::*;

pub mod provider_native_exchange;
pub use provider_native_exchange::*;
