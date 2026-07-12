use crate::orchestration::product::command::*;
use serde_json::Value;
use std::fs;
use std::path::Path;

fn fixture(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/orchestration-command-journey")
        .join(name);
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

#[test]
fn journey_and_mutation_matrices_are_explicit_and_non_claiming() {
    let journey = fixture("journey-contract.json");
    assert_eq!(journey["claim_effect"], "none");
    assert_eq!(journey["scenarios"].as_array().unwrap().len(), 3);
    let controls = fixture("mutation-controls.json");
    assert_eq!(controls["claim_effect"], "none");
    let controls = controls["controls"].as_array().unwrap();
    for required in [
        "unknown-worker",
        "stale-candidate",
        "expired-lease",
        "orphaned-lease",
        "ambiguous-head",
        "action-substitution",
        "whole-view-deserialization",
        "current-head-ready-node-substitution",
        "finding-membership-substitution",
        "root-action-membership-injection",
        "self-consistent-state-id",
        "stale-snapshot-head-mix",
        "workspace-ancestor-swap",
        "symlink-root",
        "hardlink-journal",
        "special-file-root",
        "oversized-journal",
        "non-echo-diagnostic",
    ] {
        assert!(controls.iter().any(|value| value == required), "{required}");
    }
}

#[test]
fn false_pass_offers_never_self_promote_to_public_or_journey_proof() {
    for offer in [
        EvidenceOffer::UnitTest,
        EvidenceOffer::FixtureReceipt,
        EvidenceOffer::SerializedPermit,
        EvidenceOffer::GeneratedRow,
        EvidenceOffer::InProcessSimulation,
    ] {
        assert_eq!(
            classify_evidence(offer),
            EvidenceDisposition::SupportingOnly
        );
    }
    for offer in [
        EvidenceOffer::RootIntegratedPublicCommand,
        EvidenceOffer::FreshProcessObservation,
    ] {
        assert_eq!(
            classify_evidence(offer),
            EvidenceDisposition::RequiresIndependentRootReconciliation
        );
    }
    let matrix = fixture("false-pass-controls.json");
    assert_eq!(matrix["claim_effect"], "none");
}
