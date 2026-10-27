//! Private offline entry point. Exit 2 means verification ran but release is blocked.
use pt_contracts::*;
use serde::de::DeserializeOwned;
use std::{io::Read, path::Path};
fn json<T: DeserializeOwned>(p: &str) -> Result<T, Box<dyn std::error::Error>> {
    let mut b = Vec::new();
    std::fs::File::open(p)?
        .take(4 * 1024 * 1024 + 1)
        .read_to_end(&mut b)?;
    if b.len() > 4 * 1024 * 1024 {
        return Err("private JSON exceeds 4 MiB".into());
    }
    Ok(serde_json::from_slice(&b)?)
}
fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 5 {
        eprintln!(
            "usage: verify-acceptance PACKAGE_ROOT PRIVATE_DOCUMENT_JSON PRIVATE_POLICY_JSON PRIVATE_EVIDENCE_JSON PRIVATE_INPUTS_ROOT"
        );
        std::process::exit(1)
    }
    let run = || -> Result<_, Box<dyn std::error::Error>> {
        let d: PlaybackDocument = json(&args[1])?;
        let p: AcceptanceVerificationPolicy = json(&args[2])?;
        let evidence: Vec<AcceptanceEvidenceProposal> = json(&args[3])?;
        let v = verify_acceptance(&d, Path::new(&args[0]), &p, &evidence, Path::new(&args[4]))?;
        v.recheck_inputs()?;
        v.report().validate(&acceptance_binding_checker())?;
        Ok(v)
    };
    match run() {
        Ok(v) => {
            println!("{}", serde_json::to_string(v.report()).unwrap());
            std::process::exit(if v.report().production_approved { 0 } else { 2 })
        }
        Err(e) => {
            eprintln!("private acceptance verification failed: {e}");
            std::process::exit(1)
        }
    }
}
