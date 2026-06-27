use crate::audit::contract::Failure;
use serde_json::Value;
use std::path::{Path, PathBuf};

pub(crate) fn check(receipt: &Value, root: &Path, out: &mut Vec<Failure>) {
    digest_binding_checks(receipt, root, out);
    scalar_authority_checks(receipt, out);
    report_checks(receipt, root, out);
}

fn scalar_authority_checks(receipt: &Value, out: &mut Vec<Failure>) {
    for (bad, code) in [
        (
            str_field(receipt, "tool_version").is_empty(),
            "coverage_receipt_tool_version_missing",
        ),
        (
            str_field(receipt, "workspace_root") != "/repo",
            "coverage_receipt_workspace_mismatch",
        ),
        (
            receipt.get("command_exit").and_then(Value::as_i64) != Some(0),
            "coverage_command_failed",
        ),
        (
            str_field(receipt, "generated_by") != "coverage-command",
            "coverage_receipt_not_tool_generated",
        ),
        (
            str_field(receipt, "percent_source") != "machine_readable_report",
            "coverage_percent_from_prose",
        ),
    ] {
        if bad {
            out.push(Failure::new(
                "coverage-proof-policy",
                code,
                str_field(receipt, "claim_id"),
            ));
        }
    }
}

fn digest_binding_checks(receipt: &Value, root: &Path, out: &mut Vec<Failure>) {
    let manifest_path = coverage_manifest_path(root);
    let command_path = coverage_command_path(root);
    let Ok(manifest) = crate::json_boundary::read_json(&manifest_path) else {
        return;
    };
    for (actual, expected, code) in [
        (
            crate::claim_semantics::coverage::digests::source_tree_digest(root, &manifest),
            str_field(receipt, "source_tree_digest"),
            "coverage_receipt_source_digest_mismatch",
        ),
        (
            crate::digest::file(&manifest_path),
            str_field(receipt, "coverage_manifest_digest"),
            "coverage_receipt_manifest_digest_mismatch",
        ),
        (
            crate::digest::file(&command_path),
            str_field(receipt, "coverage_command_digest"),
            "coverage_receipt_command_digest_mismatch",
        ),
        (
            crate::claim_semantics::coverage::digests::changed_files_digest(root, &manifest),
            str_field(receipt, "changed_files_digest"),
            "coverage_receipt_changed_files_digest_mismatch",
        ),
    ] {
        if actual.as_deref() != Ok(expected.as_str()) {
            out.push(Failure::new("coverage-proof-policy", code, expected));
        }
    }
}

fn report_checks(receipt: &Value, root: &Path, out: &mut Vec<Failure>) {
    let report = receipt
        .get("machine_readable_report")
        .unwrap_or(&Value::Null);
    let path = str_field(report, "path");
    let digest = str_field(report, "digest");
    let Ok(resolved) = crate::package::inventory::resolve(root, &path) else {
        out.push(Failure::new(
            "coverage-proof-policy",
            "coverage_report_missing",
            path,
        ));
        return;
    };
    let Ok(actual) = crate::digest::file(&resolved) else {
        out.push(Failure::new(
            "coverage-proof-policy",
            "coverage_report_missing",
            path,
        ));
        return;
    };
    if actual != digest {
        out.push(Failure::new(
            "coverage-proof-policy",
            "coverage_report_digest_mismatch",
            path.clone(),
        ));
    }
    if crate::json_boundary::read_json(&resolved).is_err() {
        out.push(Failure::new(
            "coverage-proof-policy",
            "coverage_report_not_machine_readable",
            path,
        ));
    }
}

fn coverage_manifest_path(root: &Path) -> PathBuf {
    let target = root.join(".harness/coverage-manifest.json");
    if target.is_file() {
        target
    } else {
        root.join("templates/.harness/coverage-manifest.json")
    }
}

fn coverage_command_path(root: &Path) -> PathBuf {
    let target = root.join(".harness/coverage-command");
    if target.is_file() {
        target
    } else {
        root.join("templates/.harness/coverage-command")
    }
}

fn str_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}
