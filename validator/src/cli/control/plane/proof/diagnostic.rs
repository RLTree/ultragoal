use crate::cli::control::plane::types::ControlOperation;
use serde_json::Value;

pub(super) fn failure_value(operation: ControlOperation, failures: &[String]) -> Value {
    if failures.is_empty() {
        return Value::Null;
    }
    let law = failure_law(operation);
    serde_json::json!({
        "id": format!("{}_evidence_not_satisfied", operation.id()),
        "law_id": law,
        "gate_id": "89",
        "check_id": law,
        "failed_invariant": "CLI control-plane claims require current same-candidate typed evidence",
        "observed_value": failures.join(" | "),
        "expected_value": "all_required_cli_authority_evidence_passes_for_current_candidate",
        "repair_class": "deterministic_enforcement",
        "rerun_command": rerun_command(operation),
        "claim_ceiling_impact": "completion_package_review_release_update_goal_withheld",
        "source_install_cache_impact": "source_only_proof_cannot_support_install_cache_app_registry_claims",
        "severity": "hard_blocker",
        "determinism": "deterministic"
    })
}

pub(super) fn notes(pass: bool, failures: &[String]) -> String {
    if pass {
        "CLI authority evidence is current, self-hosted, and same-candidate.".to_string()
    } else {
        format!(
            "Fail-closed CLI authority receipt. Evidence failures: {}",
            failures.join("; ")
        )
    }
}

fn failure_law(operation: ControlOperation) -> &'static str {
    if matches!(operation, ControlOperation::SelfUpdateGoalEligibility) {
        "cli-self-law-compliance"
    } else {
        "cli-control-plane-authority"
    }
}

fn rerun_command(operation: ControlOperation) -> &'static str {
    match operation {
        ControlOperation::RegistryProbe => {
            "ultragoal registry probe --receipt validation_artifacts/cli/registry-probe-receipt.json"
        }
        ControlOperation::AppSurfaceProbe => {
            "ultragoal app-surface probe --receipt validation_artifacts/cli/app-surface-probe-receipt.json"
        }
        ControlOperation::SelfUpdateGoalEligibility => {
            "ultragoal self update-goal eligibility --receipt validation_artifacts/cli/self-law-receipt.json"
        }
        _ => {
            "ultragoal update-goal eligibility --receipt validation_artifacts/cli/update-goal-eligibility.json"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proof_helpers_return_null_failure_and_pass_notes_for_empty_evidence() {
        let failures = Vec::new();
        assert_eq!(
            failure_value(ControlOperation::UpdateGoalEligibility, &failures),
            Value::Null
        );
        assert!(notes(true, &failures).contains("self-hosted"));
        assert_eq!(
            failure_law(ControlOperation::SelfUpdateGoalEligibility),
            "cli-self-law-compliance"
        );
        assert_eq!(
            failure_law(ControlOperation::PacketVerify),
            "cli-control-plane-authority"
        );
        assert_eq!(
            rerun_command(ControlOperation::RegistryProbe),
            "ultragoal registry probe --receipt validation_artifacts/cli/registry-probe-receipt.json"
        );
        assert_eq!(
            rerun_command(ControlOperation::AppSurfaceProbe),
            "ultragoal app-surface probe --receipt validation_artifacts/cli/app-surface-probe-receipt.json"
        );
    }
}
