//! Trusted player consumer pinning. Runtime persistence grants authority.
//!
//! Minimum policy/profile/catalog/acceptance consumer interface the player
//! adopts before genuine chapter media exists. Carries only identity
//! bindings; absent evidence stays blocking because zero-hash placeholders
//! and missing identities are rejected, and no acceptance outcome is
//! expressed here at all.
use crate::*;
use serde::{Deserialize, Serialize};

/// 64 zero hex digits. Never a trusted identity.
pub const ZERO_DIGEST: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrustedPlayerLoadV1 {
    pub schema_version: u32,
    pub profile_id: Id,
    pub checker_sha256: Digest,
    pub acceptance_report_sha256: Digest,
    pub catalog_entry_sha256: Digest,
    pub release_id: Id,
    pub allow_diagnostic: bool,
}

impl TrustedPlayerLoadV1 {
    /// Independent shared-verifier entry point. Verifies pinning shape and
    /// rejects placeholder or missing trusted identities. Grants no
    /// acceptance, dispatch, publication or media authority.
    pub fn validate(&self) -> ContractResult<()> {
        require(
            self.schema_version == 1,
            "version",
            "schema_version",
            "expected trusted player load version 1",
        )?;
        id(&self.profile_id, "trusted_load.profile_id")?;
        id(&self.release_id, "trusted_load.release_id")?;
        for (field, value) in [
            ("trusted_load.checker_sha256", &self.checker_sha256),
            (
                "trusted_load.acceptance_report_sha256",
                &self.acceptance_report_sha256,
            ),
            (
                "trusted_load.catalog_entry_sha256",
                &self.catalog_entry_sha256,
            ),
        ] {
            hash(value, field)?;
            require(
                value != ZERO_DIGEST,
                "trusted_identity",
                field,
                "trusted identities must never be zero-hash placeholders",
            )?;
        }
        Ok(())
    }
}
