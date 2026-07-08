use serde_json::Value;
use std::path::Path;

#[path = "receipt_fields.rs"]
mod receipt_fields;
use super::target_dir::{self, TargetDirStatus};

const ROUTINE_BLOCKED: &[&str] = &[
    "complete_coverage",
    "completion",
    "package_readiness",
    "review_readiness",
    "release_readiness",
    "final_packet_correctness",
    "update_goal_eligibility",
    "app_registry_or_reviewer_exposure",
];

pub(super) fn failures(root: &Path, receipt: &Path, candidate: &str) -> Vec<String> {
    let path = match receipt_fields::resolve_receipt(root, receipt) {
        Ok(path) => path,
        Err(err) => return vec![format!("coverage_routine_receipt_path_invalid:{err}")],
    };
    let receipt = match crate::json_boundary::read_json(&path) {
        Ok(value) => value,
        Err(err) => {
            return vec![format!(
                "coverage_routine_receipt_missing_or_malformed:{err}"
            )];
        }
    };
    let mut out = Vec::new();
    scalar_failures(root, &receipt, candidate, &mut out);
    lineage_failures(root, &receipt, &mut out);
    routine_claim_failures(&receipt, &mut out);
    out
}

fn scalar_failures(root: &Path, receipt: &Value, candidate: &str, out: &mut Vec<String>) {
    for (bad, code) in [
        (
            receipt_fields::string(receipt, "schema") != "harness-ultragoal.coverage-receipt.v1",
            "coverage_routine_receipt_malformed",
        ),
        (
            receipt_fields::string(receipt, "command").is_empty(),
            "coverage_routine_command_missing",
        ),
        (
            receipt_fields::string(receipt, "tool").is_empty(),
            "coverage_routine_tool_missing",
        ),
        (
            receipt_fields::string(receipt, "tool_version").is_empty(),
            "coverage_routine_tool_version_missing",
        ),
        (
            receipt_fields::string(receipt, "cargo_version").is_empty(),
            "coverage_routine_cargo_version_missing",
        ),
        (
            receipt_fields::string(receipt, "rustc_version").is_empty(),
            "coverage_routine_rustc_version_missing",
        ),
        (
            receipt_fields::string(receipt, "coverage_cache_class").is_empty(),
            "coverage_routine_cache_class_missing",
        ),
        (
            receipt.get("command_exit").and_then(Value::as_i64) != Some(0),
            "coverage_routine_command_failed",
        ),
    ] {
        if bad {
            out.push(code.to_string());
        }
    }
    match target_dir::status(
        root,
        &receipt_fields::string(receipt, "coverage_target_dir"),
    ) {
        TargetDirStatus::Missing => out.push("coverage_routine_target_dir_missing".to_string()),
        TargetDirStatus::NotIsolated => {
            out.push("coverage_routine_target_dir_not_isolated".to_string())
        }
        TargetDirStatus::Isolated => {}
    }
    if receipt
        .pointer("/target_revision/kind")
        .and_then(Value::as_str)
        != Some("package_digest")
    {
        out.push("coverage_routine_target_kind_mismatch".to_string());
    }
    if receipt
        .pointer("/target_revision/value")
        .and_then(Value::as_str)
        != Some(candidate)
    {
        out.push("coverage_routine_current_candidate_digest_mismatch".to_string());
    }
    if receipt_fields::array_strings(receipt, "target_paths").is_empty() {
        out.push("coverage_routine_missing_target_paths".to_string());
    }
    if receipt_fields::array_strings(receipt, "measured_dimensions").is_empty() {
        out.push("coverage_routine_missing_dimensions".to_string());
    }
}

fn lineage_failures(root: &Path, receipt: &Value, out: &mut Vec<String>) {
    let manifest_path = receipt_fields::coverage_manifest_path(root);
    let command_path = receipt_fields::coverage_command_path(root);
    let manifest = match crate::json_boundary::read_json(&manifest_path) {
        Ok(value) => value,
        Err(err) => {
            out.push(format!(
                "coverage_routine_manifest_missing_or_malformed:{err}"
            ));
            return;
        }
    };
    compare_file_digest(
        &manifest_path,
        receipt_fields::string(receipt, "coverage_manifest_digest"),
        "coverage_routine_manifest_digest_mismatch",
        out,
    );
    compare_file_digest(
        &command_path,
        receipt_fields::string(receipt, "coverage_command_digest"),
        "coverage_routine_command_digest_mismatch",
        out,
    );
    compare_result(
        crate::claim_semantics::coverage::digests::source_tree_digest(root, &manifest),
        receipt_fields::string(receipt, "source_tree_digest"),
        "coverage_routine_source_tree_digest_mismatch",
        out,
    );
    let manifest_targets = receipt_fields::array_strings(&manifest, "required_target_paths");
    if receipt_fields::sorted(receipt_fields::array_strings(receipt, "target_paths"))
        != receipt_fields::sorted(manifest_targets)
    {
        out.push("coverage_routine_manifest_target_gap".to_string());
    }
    let manifest_dims = manifest
        .get("required_measured_dimensions_per_root")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|row| receipt_fields::array_strings(row, "dimensions"))
        .collect::<Vec<_>>();
    if receipt_fields::sorted(receipt_fields::array_strings(
        receipt,
        "measured_dimensions",
    )) != receipt_fields::sorted(manifest_dims)
    {
        out.push("coverage_routine_required_dimension_missing".to_string());
    }
    let lineage = receipt_fields::string(receipt, "boundary_lineage_digest");
    if lineage.is_empty() {
        out.push("coverage_routine_boundary_lineage_missing".to_string());
    }
    if lineage == receipt_fields::string(receipt, "source_tree_digest") {
        out.push("coverage_routine_boundary_lineage_not_independent".to_string());
    }
    if receipt_fields::string(receipt, "equivalence_status") != "verified_current_input_equivalent"
    {
        out.push("coverage_routine_equivalence_unverified".to_string());
    }
    if receipt
        .get("coverage")
        .and_then(|value| value.get("percent"))
        .is_none()
    {
        out.push("coverage_routine_percent_missing".to_string());
    }
    if receipt
        .get("uncovered_records")
        .and_then(Value::as_array)
        .is_none()
    {
        out.push("coverage_routine_uncovered_records_missing".to_string());
    }
}

fn routine_claim_failures(receipt: &Value, out: &mut Vec<String>) {
    if receipt_fields::string(receipt, "claim_ceiling") != "routine_repair_only" {
        out.push("coverage_routine_claim_ceiling_not_routine_only".to_string());
    }
    if receipt_fields::array_contains(receipt, "supported_claim_classes", "complete_coverage") {
        out.push("coverage_routine_supports_complete_coverage".to_string());
    }
    for blocked in ROUTINE_BLOCKED {
        if !receipt_fields::array_contains(receipt, "blocked_claim_classes", blocked) {
            out.push(format!("coverage_routine_blocked_claim_missing:{blocked}"));
        }
    }
}

fn compare_file_digest(path: &Path, expected: String, code: &str, out: &mut Vec<String>) {
    compare_result(crate::digest::file(path), expected, code, out);
}

fn compare_result(
    result: Result<String, String>,
    expected: String,
    code: &str,
    out: &mut Vec<String>,
) {
    match result {
        Ok(actual) if actual == expected => {}
        Ok(_) => out.push(code.to_string()),
        Err(err) => out.push(format!("{code}:{err}")),
    }
}
