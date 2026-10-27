//! Shared wire records. Validation never grants production acceptance.
pub mod common;
pub mod services;
pub use common::*;
pub use services::*;
pub mod provider_metadata;
pub use provider_metadata::*;

pub const CONTRACT_REVISION: &str = "pt-contracts-v1.0.0";
pub const SERVICE_REVISION: &str = "pt-services-v1.5.0";
pub const CAPABILITY: &str = "paper-player-v1";

pub mod playback;
pub mod validate;
pub use playback::*;
pub use validate::*;
pub mod acceptance;
pub use acceptance::*;

pub mod provider_service;
pub use provider_service::*;
