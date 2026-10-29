//! Public export boundary. Runtime persistence grants authority.
//!
//! Executing verifier for the public-export-boundary check: an export
//! manifest is releasable only when every requested path is exactly
//! allowlisted, safe, outside private roots, byte-identical to the bound
//! digest, and not byte-identical to a known private record. Renamed
//! private-content aliases, unsafe paths and stale inventories fail.
//! Matrix wiring stays pending; this precursor grants no release authority.
use crate::*;
use serde::{Deserialize, Serialize};

/// Path segments that are never public, even if allowlisted.
const PRIVATE_SEGMENTS: [&str; 4] = ["private", "raw", "journal", "accounting"];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicExportEntryV1 {
    pub path: String,
    pub sha256: Digest,
    pub bytes: u64,
    pub media_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicExportAllowlistV1 {
    pub schema_version: u32,
    pub entries: Vec<PublicExportEntryV1>,
}

fn private_root(path: &str) -> bool {
    path.split('/').any(|s| PRIVATE_SEGMENTS.contains(&s))
}

impl PublicExportAllowlistV1 {
    /// Independent shared-verifier entry point. `files` carries the exact
    /// bytes offered for each requested path; `known_private_digests`
    /// carries digests of private records that must never leave, whatever
    /// name they wear. Verifies, never releases.
    pub fn verify_public_export(
        &self,
        files: &[(&str, &[u8])],
        known_private_digests: &[Digest],
    ) -> ContractResult<()> {
        require(
            self.schema_version == 1,
            "version",
            "schema_version",
            "expected public export allowlist version 1",
        )?;
        require(
            !self.entries.is_empty(),
            "export_boundary",
            "entries",
            "an empty allowlist exports nothing",
        )?;
        for entry in &self.entries {
            require(
                safe_path(&entry.path) && !private_root(&entry.path),
                "export_boundary",
                "path",
                "allowlist must name safe public files only",
            )?;
            hash(&entry.sha256, "export_boundary.allowlist")?;
        }
        for (path, bytes) in files.iter().copied() {
            require(
                safe_path(path) && !private_root(path),
                "export_boundary",
                path,
                "unsafe or private path can never export",
            )?;
            let entry = self
                .entries
                .iter()
                .find(|e| e.path == *path)
                .ok_or(ContractError {
                    code: "export_boundary".into(),
                    field: (*path).into(),
                    message: "path is not allowlisted for export".into(),
                })?;
            let actual = digest(bytes);
            require(
                actual == entry.sha256 && bytes.len() as u64 == entry.bytes,
                "export_boundary",
                path,
                "stale inventory: bytes differ from the allowlisted digest",
            )?;
            require(
                !known_private_digests.iter().any(|d| d == &actual),
                "export_boundary",
                path,
                "bytes match a known private record",
            )?;
        }
        Ok(())
    }
}
