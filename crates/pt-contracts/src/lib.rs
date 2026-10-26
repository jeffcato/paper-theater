//! Shared wire records. Validation never grants production acceptance.
pub mod common;
pub mod services;
pub use common::*;
pub use services::*;

pub const CONTRACT_REVISION: &str = "pt-contracts-v1.0.0";
pub const SERVICE_REVISION: &str = "pt-services-v1.0.0";
pub const CAPABILITY: &str = "paper-player-v1";

pub mod playback;
pub mod validate;
pub use playback::*;
pub use validate::*;
