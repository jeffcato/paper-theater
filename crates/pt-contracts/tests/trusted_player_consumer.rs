use pt_contracts::*;
#[path = "support/trusted_player_cases.rs"]
mod cases;

#[test]
fn cross_language_trusted_player_cases_agree() {
    for row in cases::records() {
        assert_eq!(
            cases::check(&row).is_ok(),
            row["valid"] == true,
            "{}",
            row["id"]
        );
    }
}

#[test]
fn exact_trusted_player_pinning_passes() {
    cases::fixture().validate().unwrap();
}

#[test]
fn zero_hash_placeholders_are_rejected() {
    for field in [
        "checker_sha256",
        "acceptance_report_sha256",
        "catalog_entry_sha256",
    ] {
        let mut load = cases::fixture();
        let digest = match field {
            "checker_sha256" => &mut load.checker_sha256,
            "acceptance_report_sha256" => &mut load.acceptance_report_sha256,
            _ => &mut load.catalog_entry_sha256,
        };
        *digest = ZERO_DIGEST.into();
        assert!(load.validate().is_err(), "{field}");
    }
}

#[test]
fn missing_trusted_identities_are_rejected() {
    let mut load = cases::fixture();
    load.profile_id = "".into();
    assert!(load.validate().is_err());
    let mut load = cases::fixture();
    load.release_id = "".into();
    assert!(load.validate().is_err());
}
