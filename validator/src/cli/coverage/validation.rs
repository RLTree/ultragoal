use serde_json::Value;
use std::path::Path;

#[path = "receipt_fields.rs"]
mod receipt_fields;

const REQUIRED_BLOCKED: &[&str] = &[
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
        Err(err) => return vec![format!("coverage_receipt_path_invalid:{err}")],
    };
    let receipt = match crate::json_boundary::read_json(&path) {
        Ok(value) => value,
        Err(err) => return vec![format!("coverage_receipt_missing_or_malformed:{err}")],
    };
    let mut out = Vec::new();
    scalar_failures(root, &receipt, candidate, &mut out);
    digest_failures(root, &receipt, &mut out);
    report_failures(root, &receipt, &mut out);
    completion_failures(&receipt, &mut out);
    out
}

fn scalar_failures(root: &Path, receipt: &Value, candidate: &str, out: &mut Vec<String>) {
    for (bad, code) in [
        (
            receipt_fields::string(receipt, "schema") != "harness-ultragoal.coverage-receipt.v1",
            "coverage_receipt_malformed",
        ),
        (
            receipt_fields::string(receipt, "command").is_empty(),
            "coverage_command_missing",
        ),
        (
            receipt_fields::string(receipt, "tool").is_empty(),
            "coverage_tool_missing",
        ),
        (
            receipt_fields::string(receipt, "tool_version").is_empty(),
            "coverage_receipt_tool_version_missing",
        ),
        (
            receipt_fields::string(receipt, "generated_by") != "coverage-command",
            "coverage_receipt_not_tool_generated",
        ),
        (
            receipt_fields::string(receipt, "percent_source") != "machine_readable_report",
            "coverage_percent_from_prose",
        ),
        (
            receipt.get("command_exit").and_then(Value::as_i64) != Some(0),
            "coverage_command_failed",
        ),
    ] {
        if bad {
            out.push(code.to_string());
        }
    }
    let workspace = receipt_fields::string(receipt, "workspace_root");
    let root_string = root
        .canonicalize()
        .unwrap_or_else(|_| root.to_path_buf())
        .to_string_lossy()
        .to_string();
    if workspace != "/repo" && workspace != root_string {
        out.push("coverage_receipt_workspace_mismatch".to_string());
    }
    let target = receipt.get("target_revision").unwrap_or(&Value::Null);
    if receipt_fields::string(target, "kind") != "package_digest" {
        out.push("coverage_receipt_target_kind_mismatch".to_string());
    }
    if receipt_fields::string(target, "value") != candidate {
        out.push(format!(
            "coverage_receipt_target_digest_mismatch:expected={candidate} actual={}",
            receipt_fields::string(target, "value")
        ));
    }
    if receipt_fields::array_strings(receipt, "target_paths").is_empty() {
        out.push("coverage_claim_missing_target_paths".to_string());
    }
    if receipt_fields::array_strings(receipt, "measured_dimensions").is_empty() {
        out.push("coverage_claim_missing_dimensions".to_string());
    }
}

fn digest_failures(root: &Path, receipt: &Value, out: &mut Vec<String>) {
    let manifest_path = receipt_fields::coverage_manifest_path(root);
    let command_path = receipt_fields::coverage_command_path(root);
    let manifest = match crate::json_boundary::read_json(&manifest_path) {
        Ok(value) => value,
        Err(err) => {
            out.push(format!("coverage_manifest_missing_or_malformed:{err}"));
            return;
        }
    };
    compare_file_digest(
        &manifest_path,
        receipt_fields::string(receipt, "coverage_manifest_digest"),
        "coverage_receipt_manifest_digest_mismatch",
        out,
    );
    compare_file_digest(
        &command_path,
        receipt_fields::string(receipt, "coverage_command_digest"),
        "coverage_receipt_command_digest_mismatch",
        out,
    );
    compare_result(
        crate::claim_semantics::coverage::digests::source_tree_digest(root, &manifest),
        receipt_fields::string(receipt, "source_tree_digest"),
        "coverage_receipt_source_digest_mismatch",
        out,
    );
    compare_result(
        crate::claim_semantics::coverage::digests::changed_files_digest(root, &manifest),
        receipt_fields::string(receipt, "changed_files_digest"),
        "coverage_receipt_changed_files_digest_mismatch",
        out,
    );
    let manifest_targets = receipt_fields::array_strings(&manifest, "required_target_paths");
    if receipt_fields::sorted(receipt_fields::array_strings(receipt, "target_paths"))
        != receipt_fields::sorted(manifest_targets)
    {
        out.push("coverage_manifest_target_gap".to_string());
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
        out.push("coverage_required_dimension_missing".to_string());
    }
}

fn report_failures(root: &Path, receipt: &Value, out: &mut Vec<String>) {
    let report = receipt
        .get("machine_readable_report")
        .unwrap_or(&Value::Null);
    let report_path = receipt_fields::string(report, "path");
    if report_path.is_empty() {
        out.push("coverage_report_missing".to_string());
        return;
    }
    let resolved = match crate::package::inventory::resolve(root, &report_path) {
        Ok(path) => path,
        Err(err) => {
            out.push(format!("coverage_report_missing:{err}"));
            return;
        }
    };
    if crate::json_boundary::read_json(&resolved).is_err() {
        out.push("coverage_report_not_machine_readable".to_string());
    }
    compare_file_digest(
        &resolved,
        receipt_fields::string(report, "digest"),
        "coverage_report_digest_mismatch",
        out,
    );
}

fn completion_failures(receipt: &Value, out: &mut Vec<String>) {
    if receipt.pointer("/coverage/policy").and_then(Value::as_str) != Some("100_percent_required") {
        out.push("coverage_ratchet_presented_as_complete".to_string());
    }
    if receipt.pointer("/coverage/percent").and_then(Value::as_f64) != Some(100.0) {
        out.push("coverage_claim_uncovered_code".to_string());
    }
    let uncovered = receipt
        .get("uncovered_records")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !uncovered.is_empty() {
        out.push("coverage_claim_uncovered_code".to_string());
        out.push(uncovered_summary(&uncovered));
    }
    if receipt_fields::string(receipt, "claim_ceiling") != "supports_complete_coverage_claim" {
        out.push("coverage_ratchet_presented_as_complete".to_string());
    }
    if !receipt_fields::array_contains(receipt, "supported_claim_classes", "complete_coverage") {
        out.push("coverage_ratchet_presented_as_complete".to_string());
    }
    for claim in REQUIRED_BLOCKED {
        if !receipt_fields::array_contains(receipt, "blocked_claim_classes", claim) {
            out.push(format!("coverage_blocked_claim_missing:{claim}"));
        }
    }
}

fn uncovered_summary(records: &[Value]) -> String {
    let first = records
        .iter()
        .take(5)
        .map(|record| {
            format!(
                "{} ({})",
                receipt_fields::string(record, "path"),
                receipt_fields::string(record, "reason")
            )
        })
        .collect::<Vec<_>>()
        .join(" | ");
    format!(
        "coverage_uncovered_records:total={} first={first}",
        records.len()
    )
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
