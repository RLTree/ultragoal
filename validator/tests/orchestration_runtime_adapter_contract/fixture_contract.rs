use crate::orchestration::product::command::{
    EvidenceDisposition, EvidenceOffer, classify_evidence,
};
use serde_json::Value;
use std::fs;
use std::path::Path;

fn fixture(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/orchestration-runtime-adapter")
        .join(name);
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

#[test]
fn journey_and_mutation_matrices_are_explicit_and_non_claiming() {
    let journey = fixture("journey-contract.json");
    assert_eq!(journey["claim_effect"], "none");
    let scenarios = journey["scenarios"].as_array().unwrap();
    assert_eq!(scenarios.len(), 3);
    assert!(scenarios.iter().all(|scenario| {
        scenario["processes"].as_array().is_some_and(|steps| {
            steps.len() == 4
                && steps[1] == "inspect-and-authorize"
                && steps[2] == "execute"
                && steps[3] == "reopen"
        })
    }));
    let reconciliation = scenarios
        .iter()
        .find(|scenario| scenario["id"] == "ambiguous-effect-reconciliation")
        .unwrap();
    assert_eq!(
        reconciliation["decision_binding"],
        "complete-effect-resolution"
    );

    let controls = fixture("mutation-controls.json");
    assert_eq!(controls["claim_effect"], "none");
    let controls = controls["controls"].as_array().unwrap();
    for required in [
        "stale-current-view",
        "stale-interrupted-view",
        "action-substitution",
        "request-head-substitution",
        "request-target-substitution",
        "request-lease-substitution",
        "request-result-substitution",
        "request-operation-substitution",
        "request-tick-substitution",
        "request-live-workers-substitution",
        "request-recovered-binding-substitution",
        "expired-permit",
        "wrong-operation-permit",
        "forged-permit",
        "effect-evidence-substitution",
        "effect-outcome-substitution",
        "effect-receipt-operation-substitution",
        "effect-receipt-class-substitution",
        "effect-receipt-target-substitution",
        "effect-receipt-digest-substitution",
        "permit-v1-downgrade",
        "permit-decision-binding-missing",
        "action-reconcile-cross-use",
        "same-view-differently-bound-decisions",
        "concurrent-recovery",
        "replay-after-recovery",
        "workspace-ancestor-swap",
        "symlink-root",
        "hardlink-journal",
        "fifo-root-entry",
        "socket-root-entry",
        "oversized-journal",
        "non-echo-diagnostic",
        "recursive-zero-write-read",
    ] {
        assert!(controls.iter().any(|value| value == required), "{required}");
    }
}

#[test]
fn supporting_evidence_never_self_promotes_to_root_or_runtime_proof() {
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
    let controls = fixture("false-pass-controls.json");
    assert_eq!(controls["claim_effect"], "none");
    for withheld in [
        "root-dispatch",
        "real-permit-issuance",
        "installed-fresh-binary",
        "public-command",
        "node-closure",
        "readiness",
        "release",
        "completion",
    ] {
        assert!(
            controls["withheld"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value == withheld)
        );
    }
    for refused in [
        "pre-decision-root-action-request",
        "permit-v1",
        "permit-without-decision-binding",
        "permit-bound-to-a-different-resolution",
    ] {
        assert!(
            controls["refused_as_authority"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value == refused)
        );
    }
}
