use crate::cli::control::plane::RECEIPT_SCHEMA;
use crate::cli::control::plane::proof;
use crate::cli::control::plane::types::{ControlOperation, REQUIRED_COMMANDS};
use serde_json::{Value, json};

#[cfg(test)]
pub(crate) fn receipt_from_evidence(
    package_digest: String,
    operation: ControlOperation,
    mut evidence_failures: Vec<String>,
) -> Value {
    // Raw constructor calls are never update_goal authority. Only the
    // production path can pass after dereferencing current same-candidate proof.
    if evidence_failures.is_empty() {
        evidence_failures.push("cli_control_plane_transactional_green_path_not_proven".to_string());
    }
    fail_receipt(package_digest, operation, evidence_failures)
}

pub(crate) fn receipt_from_production_evidence(
    package_digest: String,
    operation: ControlOperation,
    evidence_failures: Vec<String>,
) -> Value {
    if evidence_failures.is_empty() {
        pass_receipt(package_digest, operation)
    } else {
        fail_receipt(package_digest, operation, evidence_failures)
    }
}

fn pass_receipt(package_digest: String, operation: ControlOperation) -> Value {
    json!({
        "schema": RECEIPT_SCHEMA,
        "schema_version": "v1",
        "issuer": {
            "tool": "ultragoal",
            "authority": "cli_control_plane",
            "compatibility_binary": "ultragoal-validator",
            "self_law_state": "self_hosted"
        },
        "generated_at": crate::audit::clock::now_iso(),
        "root": ".",
        "candidate_digest": package_digest,
        "operation": operation.id(),
        "status": "pass",
        "claim_ceiling": "supports_update_goal_eligibility",
        "blocked_claim_classes": [],
        "required_evidence": required_evidence(operation),
        "failure": Value::Null,
        "command_surface": REQUIRED_COMMANDS,
        "notes": proof::notes(true, &[])
    })
}

fn fail_receipt(
    package_digest: String,
    operation: ControlOperation,
    evidence_failures: Vec<String>,
) -> Value {
    json!({
        "schema": RECEIPT_SCHEMA,
        "schema_version": "v1",
        "issuer": {
            "tool": "ultragoal",
            "authority": "cli_control_plane",
            "compatibility_binary": "ultragoal-validator",
            "self_law_state": "transition_only"
        },
        "generated_at": crate::audit::clock::now_iso(),
        "root": ".",
        "candidate_digest": package_digest,
        "operation": operation.id(),
        "status": "fail",
        "claim_ceiling": "withheld_or_blocked",
        "blocked_claim_classes": blocked_claims(operation),
        "required_evidence": required_evidence(operation),
        "failure": proof::failure_value(operation, &evidence_failures),
        "command_surface": REQUIRED_COMMANDS,
        "notes": proof::notes(false, &evidence_failures)
    })
}

fn blocked_claims(operation: ControlOperation) -> Vec<&'static str> {
    let mut claims = vec![
        "completion",
        "package_readiness",
        "review_readiness",
        "release_readiness",
        "update_goal_eligibility",
    ];
    if matches!(
        operation,
        ControlOperation::RegistryProbe | ControlOperation::AppSurfaceProbe
    ) {
        claims.push("app_registry_or_reviewer_exposure");
    }
    claims
}

fn required_evidence(operation: ControlOperation) -> Vec<&'static str> {
    if matches!(
        operation,
        ControlOperation::RegistryProbe | ControlOperation::AppSurfaceProbe
    ) {
        return vec!["live_registry_or_reviewer_exposure_same_surface_pass"];
    }
    let mut out = vec![
        "current_red_fixture_report_status_pass",
        "coverage_100_no_uncovered_records",
        "current_cli_performance_pass",
        "current_final_packet_proof_pass",
        "live_registry_or_reviewer_exposure_same_surface_pass",
    ];
    if matches!(
        operation,
        ControlOperation::UpdateGoalEligibility | ControlOperation::SelfUpdateGoalEligibility
    ) {
        out.push("all_89_gates_and_100_stop_conditions_pass");
    }
    out
}
