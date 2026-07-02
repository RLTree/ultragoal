use super::lines;
use serde_json::json;
use std::path::Path;

#[test]
fn pass_receipt_without_supported_claims_does_not_overstate_registry_proof() {
    let value = json!({
        "status": "pass",
        "operation": "registry_probe",
        "candidate_digest": "sha256:test",
        "run_id": "run-test",
        "correlation_id": "corr-test",
        "claim_impact": "withheld_missing_supported_claims",
        "observability": {
            "blocked_claims": ["readiness", "release", "update_goal"]
        }
    });

    let output = lines(Path::new("validation_artifacts/cli/registry.json"), &value);

    assert_eq!(output.len(), 1);
    assert!(output[0].contains("ultragoal-registry-probe pass"));
    assert!(output[0].contains("proven=none"), "{}", output[0]);
    assert!(
        !output[0].contains("live_registry_or_reviewer_exposure_same_surface_pass"),
        "{}",
        output[0]
    );
}

#[test]
fn pass_receipt_with_empty_supported_claims_does_not_overstate_registry_proof() {
    let value = json!({
        "status": "pass",
        "operation": "registry_probe",
        "candidate_digest": "sha256:test",
        "run_id": "run-test",
        "correlation_id": "corr-test",
        "claim_impact": "withheld_empty_supported_claims",
        "observability": {
            "supported_claims": [],
            "blocked_claims": ["readiness", "release", "update_goal"]
        }
    });

    let output = lines(Path::new("validation_artifacts/cli/registry.json"), &value);

    assert_eq!(output.len(), 1);
    assert!(output[0].contains("proven=none"), "{}", output[0]);
    assert!(output[0].contains("supported_claims=none"), "{}", output[0]);
}
