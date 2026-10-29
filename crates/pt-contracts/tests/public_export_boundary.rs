use pt_contracts::*;

fn allowlist() -> PublicExportAllowlistV1 {
    let body = br#"{"release":"oz-1"}"#;
    PublicExportAllowlistV1 {
        schema_version: 1,
        entries: vec![PublicExportEntryV1 {
            path: "release.json".into(),
            sha256: digest(body),
            bytes: body.len() as u64,
            media_type: "application/json".into(),
        }],
    }
}

fn records() -> Vec<(&'static str, Vec<u8>)> {
    vec![("release.json", br#"{"release":"oz-1"}"#.to_vec())]
}

#[test]
fn exact_allowlisted_public_file_passes() {
    allowlist()
        .verify_public_export(
            &[("release.json", br#"{"release":"oz-1"}"#.as_slice())],
            &[],
        )
        .unwrap();
    let rows = records();
    let borrowed: Vec<(&str, &[u8])> = rows.iter().map(|(p, b)| (*p, b.as_slice())).collect();
    allowlist().verify_public_export(&borrowed, &[]).unwrap();
}

#[test]
fn renamed_private_content_alias_is_rejected() {
    // Same bytes as no allowlisted path: exact path match required.
    assert!(
        allowlist()
            .verify_public_export(
                &[("reports.json", br#"{"release":"oz-1"}"#.as_slice())],
                &[]
            )
            .is_err()
    );
}

#[test]
fn private_roots_never_export_even_when_named() {
    for path in [
        "private/receipt.json",
        "raw/response.bin",
        "journal/entries.jsonl",
        "accounting/ledger.json",
        "release/../private/receipt.json",
        "/absolute.json",
    ] {
        assert!(
            allowlist()
                .verify_public_export(&[(path, b"{}".as_slice())], &[])
                .is_err(),
            "{path}"
        );
    }
}

#[test]
fn stale_inventory_and_wrong_bytes_are_rejected() {
    assert!(
        allowlist()
            .verify_public_export(
                &[("release.json", br#"{"release":"oz-2"}"#.as_slice())],
                &[]
            )
            .is_err()
    );
}

#[test]
fn byte_identical_private_record_is_rejected_under_any_name() {
    let secret = br#"{"private_receipt":"sentinel"}"#.as_slice();
    let private = digest(secret);
    // Not allowlisted at all: exact path match already rejects it.
    assert!(
        allowlist()
            .verify_public_export(&[("assets/receipt.svg", secret)], &[])
            .is_err()
    );
    // Even if an allowlist entry were poisoned, known private bytes stay blocked.
    let poisoned = PublicExportAllowlistV1 {
        schema_version: 1,
        entries: vec![PublicExportEntryV1 {
            path: "assets/receipt.svg".into(),
            sha256: private.clone(),
            bytes: secret.len() as u64,
            media_type: "image/svg+xml".into(),
        }],
    };
    assert!(
        poisoned
            .verify_public_export(&[("assets/receipt.svg", secret)], &[private])
            .is_err()
    );
}
