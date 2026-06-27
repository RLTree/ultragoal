use std::path::Path;

pub(crate) fn failures(root: &Path, path: &str, kind: &str) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(root.join(path)) else {
        return vec![format!("coverage_scope_surface_missing:{path}")];
    };
    let mut out = required_token_failures(path, &text);
    if kind == "completion" && !text.contains("100_percent_required") {
        out.push("coverage_full_gate_missing".to_string());
    }
    if kind == "completion" && text.contains("progress claim only") {
        out.push("coverage_fast_gate_used_for_completion".to_string());
    }
    if kind == "progress" && !text.contains("progress") {
        out.push("coverage_claim_context_missing".to_string());
    }
    out
}

fn required_token_failures(path: &str, text: &str) -> Vec<String> {
    required_tokens()
        .iter()
        .filter(|token| !text.contains(**token))
        .map(|token| format!("coverage_script_missing_contract:{path}:{token}"))
        .collect()
}

fn required_tokens() -> &'static [&'static str] {
    &[
        ".harness/coverage-manifest.json",
        ".harness/coverage-command",
        "coverage_receipt_missing",
        "coverage_command_failed",
        "coverage_manifest_digest",
        "coverage_command_digest",
        "changed_files_digest",
        "source_tree_digest",
        "machine_readable_report",
        "coverage_report_digest_mismatch",
        "coverage_receipt_manifest_digest_mismatch",
        "coverage_receipt_command_digest_mismatch",
        "coverage_receipt_changed_files_digest_mismatch",
        "coverage_receipt_source_digest_mismatch",
        "coverage_receipt_workspace_mismatch",
        "coverage_receipt_not_tool_generated",
        "coverage_percent_from_prose",
        "coverage_report_not_machine_readable",
        "coverage_target_path_nonexistent",
        "coverage_target_path_ignored_local_state",
        "coverage_required_dimension_missing",
    ]
}
