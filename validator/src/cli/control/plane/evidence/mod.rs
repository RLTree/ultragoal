use crate::cli::control::plane::types::ControlOperation;
use serde_json::{Value, json};
use std::path::Path;

mod item;
mod receipt_evaluation;
mod validate;

const SOURCE_AUDIT: &str = "validation_artifacts/ultragoal-audit/validator-receipt.json";
const RED_REPORT: &str = "validation_artifacts/ultragoal-audit/red-fixture-report.json";
const COVERAGE: &str = "validation_artifacts/coverage/coverage-receipt.json";
const CLI_PERFORMANCE: &str = "validation_artifacts/cli/performance-receipt.json";
const FINAL_PACKET: &str = "validation_artifacts/review/final-packet-proof.json";
const REGISTRY_EXPOSURE: &str =
    "validation_artifacts/ultragoal-audit/active-registry-exposure-current.json";
const TRANSACTION: &str = "validation_artifacts/cli/transactional-finalization-receipt.json";
const FIT_REPO: &str = "validation_artifacts/harness/fit-repo-receipt.json";
const PRODUCT_FITNESS: &str = "validation_artifacts/harness/product-fitness-receipt.json";
const PRODUCT_JOURNEY: &str = "validation_artifacts/harness/plugin-product-journey-receipt.json";
const STANDARDS_GARDENER: &str =
    "validation_artifacts/standards-gardener/current-standards-gardening-receipt.json";
const INSTALL_AUDIT: &str = "validation_artifacts/cli/install-audit-receipt.json";
const CACHE_AUDIT: &str = "validation_artifacts/cli/cache-audit-receipt.json";

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
        ("source_audit", SOURCE_AUDIT),
        ("red_fixture_report", RED_REPORT),
        ("coverage", COVERAGE),
        ("cli_performance", CLI_PERFORMANCE),
        ("final_packet", FINAL_PACKET),
        ("registry_exposure", REGISTRY_EXPOSURE),
        ("fit_repo", FIT_REPO),
        ("product_fitness", PRODUCT_FITNESS),
        ("product_journey", PRODUCT_JOURNEY),
        ("standards_gardener", STANDARDS_GARDENER),
        ("install_audit", INSTALL_AUDIT),
        ("cache_audit", CACHE_AUDIT),
        (
            "rust_toolchain",
            "validation_artifacts/rust/toolchain-receipt.json",
        ),
        ("rust_fast", "validation_artifacts/rust/fast-receipt.json"),
        (
            "rust_standard",
            "validation_artifacts/rust/standard-receipt.json",
        ),
        (
            "rust_release",
            "validation_artifacts/rust/release-receipt.json",
        ),
        (
            "rust_clean_proof",
            "validation_artifacts/rust/clean-proof-receipt.json",
        ),
        ("rust_watch", "validation_artifacts/rust/watch-receipt.json"),
        (
            "rust_memory",
            "validation_artifacts/rust/memory-receipt.json",
        ),
        (
            "rust_dependency",
            "validation_artifacts/rust/dependency-receipt.json",
        ),
        (
            "rust_coverage",
            "validation_artifacts/rust/coverage-receipt.json",
        ),
        (
            "rust_workspace_topology",
            "validation_artifacts/rust/workspace-topology-receipt.json",
        ),
        ("gc_plan", "validation_artifacts/gc/plan-receipt.json"),
        ("gc_dry_run", "validation_artifacts/gc/dry-run-receipt.json"),
        ("gc_apply", "validation_artifacts/gc/apply-receipt.json"),
        ("gc_verify", "validation_artifacts/gc/verify-receipt.json"),
    ];
    if matches!(
        operation,
        ControlOperation::UpdateGoalEligibility | ControlOperation::SelfUpdateGoalEligibility
    ) {
        out.push(("transactional_finalization", TRANSACTION));
    }
    out
}

pub(super) fn required_labels(operation: &str) -> &'static [&'static str] {
    match operation {
        "registry_probe" | "app_surface_probe" => &["registry_exposure"],
        "update_goal_eligibility" | "self_update_goal_eligibility" => &[
            "source_audit",
            "red_fixture_report",
            "coverage",
            "cli_performance",
            "final_packet",
            "registry_exposure",
            "fit_repo",
            "product_fitness",
            "product_journey",
            "standards_gardener",
            "install_audit",
            "cache_audit",
            "rust_toolchain",
            "rust_fast",
            "rust_standard",
            "rust_release",
            "rust_clean_proof",
            "rust_watch",
            "rust_memory",
            "rust_dependency",
            "rust_coverage",
            "rust_workspace_topology",
            "gc_plan",
            "gc_dry_run",
            "gc_apply",
            "gc_verify",
            "transactional_finalization",
        ],
        _ => &[
            "source_audit",
            "red_fixture_report",
            "coverage",
            "cli_performance",
            "final_packet",
            "registry_exposure",
            "fit_repo",
            "product_fitness",
            "product_journey",
            "standards_gardener",
            "install_audit",
            "cache_audit",
            "rust_toolchain",
            "rust_fast",
            "rust_standard",
            "rust_release",
            "rust_clean_proof",
            "rust_watch",
            "rust_memory",
            "rust_dependency",
            "rust_coverage",
            "rust_workspace_topology",
            "gc_plan",
            "gc_dry_run",
            "gc_apply",
            "gc_verify",
        ],
    }
}
