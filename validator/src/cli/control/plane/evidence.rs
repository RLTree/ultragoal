use crate::cli::control::plane::types::ControlOperation;
use serde_json::{Value, json};
use std::path::Path;

mod item;
mod validate;

const RED_REPORT: &str = "validation_artifacts/ultragoal-audit/red-fixture-report.json";
const COVERAGE: &str = "validation_artifacts/coverage/coverage-receipt.json";
const CLI_PERFORMANCE: &str = "validation_artifacts/cli/performance-receipt.json";
const FINAL_PACKET: &str = "validation_artifacts/review/final-packet-proof.json";
const REGISTRY_EXPOSURE: &str =
    "validation_artifacts/ultragoal-audit/active-registry-exposure-current.json";
const SOURCE_AUDIT: &str = "validation_artifacts/ultragoal-audit/validator-receipt.json";
const TRANSACTION: &str = "validation_artifacts/cli/transactional-finalization-receipt.json";

pub(crate) fn production(
    root: &Path,
    operation: ControlOperation,
    candidate: &str,
    operation_failures: &[String],
) -> Value {
    let items = specs(operation)
        .into_iter()
        .map(|(label, path)| item::value(root, label, path, candidate))
        .collect::<Vec<_>>();
    json!({
        "candidate_digest": candidate,
        "evaluation_mode": "production_dereferenced",
        "operation": operation.id(),
        "operation_failures": operation_failures,
        "items": items
    })
}

#[cfg(test)]
pub(crate) fn test_constructor(candidate: &str, operation: ControlOperation) -> Value {
    json!({
        "candidate_digest": candidate,
        "evaluation_mode": "test_constructor_not_authoritative",
        "operation": operation.id(),
        "operation_failures": ["cli_control_plane_transactional_green_path_not_proven"],
        "items": []
    })
}

pub(crate) fn receipt_surface_failures(value: &Value) -> Vec<String> {
    validate::receipt_surface_failures(value)
}

pub(crate) fn same_candidate_pass_failures(
    value: &Value,
    expected_candidate: &str,
    expected_operation: &str,
) -> Vec<String> {
    validate::same_candidate_pass_failures(value, expected_candidate, expected_operation)
}

pub(crate) fn same_candidate_fail_closed_failures(
    value: &Value,
    expected_candidate: &str,
    expected_operation: &str,
) -> Vec<String> {
    validate::same_candidate_fail_closed_failures(value, expected_candidate, expected_operation)
}

fn specs(operation: ControlOperation) -> Vec<(&'static str, &'static str)> {
    if matches!(
        operation,
        ControlOperation::RegistryProbe | ControlOperation::AppSurfaceProbe
    ) {
        return vec![("registry_exposure", REGISTRY_EXPOSURE)];
    }
    let mut out = vec![
        ("red_fixture_report", RED_REPORT),
        ("coverage", COVERAGE),
        ("cli_performance", CLI_PERFORMANCE),
        ("final_packet", FINAL_PACKET),
        ("registry_exposure", REGISTRY_EXPOSURE),
    ];
    if matches!(
        operation,
        ControlOperation::UpdateGoalEligibility | ControlOperation::SelfUpdateGoalEligibility
    ) {
        out.push(("source_audit", SOURCE_AUDIT));
        out.push(("transactional_finalization", TRANSACTION));
    }
    out
}

pub(super) fn required_labels(operation: &str) -> &'static [&'static str] {
    match operation {
        "registry_probe" | "app_surface_probe" => &["registry_exposure"],
        "update_goal_eligibility" | "self_update_goal_eligibility" => &[
            "red_fixture_report",
            "coverage",
            "cli_performance",
            "final_packet",
            "registry_exposure",
            "source_audit",
            "transactional_finalization",
        ],
        _ => &[
            "red_fixture_report",
            "coverage",
            "cli_performance",
            "final_packet",
            "registry_exposure",
        ],
    }
}
