mod builder_contract;
pub(crate) mod changed_files;
pub(crate) mod digests;
mod dimensions;
pub(crate) mod exclusions;
pub(crate) mod roots;
pub(crate) mod scripts;
use serde_json::Value;
use std::path::Path;

const MANIFEST: &str = "templates/.harness/coverage-manifest.json";
const COMMAND: &str = "templates/.harness/coverage-command";
const FAST: &str = "templates/scripts/check-coverage-fast";
const FULL: &str = "templates/scripts/check-coverage-full";

pub fn package_failures(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for path in [
        MANIFEST,
        COMMAND,
        FAST,
        FULL,
        "templates/COVERAGE_RECEIPT.json",
    ] {
        if !root.join(path).is_file() {
            out.push(format!("coverage_scope_surface_missing:{path}"));
        }
    }
    let manifest = match crate::json_boundary::read_json(&root.join(MANIFEST)) {
        Ok(value) => value,
        Err(err) => {
            out.push(format!("coverage_manifest_malformed:{err}"));
            return out;
        }
    };
    out.extend(value_failures_with_root(&manifest, Some(root)));
    out.extend(crate::audit::coverage::scope::scripts::failures(
        root,
        FAST,
        "source_local_iteration",
    ));
    out.extend(crate::audit::coverage::scope::scripts::failures(
        root,
        FULL,
        "completion",
    ));
    out
}

pub(crate) fn value_failures_with_root(value: &Value, root: Option<&Path>) -> Vec<String> {
    let mut out = Vec::new();
    if str_field(value, "schema") != "harness-ultragoal.coverage-manifest.v1" {
        out.push("coverage_receipt_malformed:schema".to_string());
    }
    if str_field(value, "coverage_command_path") != ".harness/coverage-command" {
        out.push("coverage_command_missing:path".to_string());
    }
    if value
        .get("required_target_paths")
        .and_then(Value::as_array)
        .is_none_or(Vec::is_empty)
    {
        out.push("coverage_claim_missing_target_paths".to_string());
    }
    out.extend(scope_authority_failures(value, root));
    out.extend(policy_section_failures(value));
    if value
        .get("required_measured_dimensions_per_root")
        .and_then(Value::as_array)
        .is_none_or(Vec::is_empty)
    {
        out.push("coverage_claim_missing_dimensions".to_string());
    }
    out.extend(crate::audit::coverage::scope::exclusions::failures(value));
    out
}

fn policy_section_failures(value: &Value) -> Vec<String> {
    let mut out = Vec::new();
    for (ptr, code) in [
        (
            "/repo_walk_policy/classify_all_nonignored_files",
            "coverage_source_unclassified",
        ),
        (
            "/repo_walk_policy/ignored_local_state_cannot_be_target",
            "coverage_target_path_ignored_local_state",
        ),
        (
            "/policy_mutation_gate/material_review_required",
            "coverage_policy_weakened_without_review",
        ),
        (
            "/receipt_freshness_binding/source_tree_digest_required",
            "coverage_receipt_source_digest_mismatch",
        ),
        (
            "/receipt_freshness_binding/manifest_digest_required",
            "coverage_receipt_manifest_digest_mismatch",
        ),
        (
            "/receipt_freshness_binding/command_digest_required",
            "coverage_receipt_command_digest_mismatch",
        ),
        (
            "/receipt_freshness_binding/changed_files_digest_required",
            "coverage_receipt_changed_files_digest_mismatch",
        ),
        (
            "/tool_generated_proof_policy/hand_written_receipts_rejected",
            "coverage_receipt_not_tool_generated",
        ),
        (
            "/tool_generated_proof_policy/prose_percent_rejected",
            "coverage_percent_from_prose",
        ),
        (
            "/fast_full_gate_split/full_required_for_completion",
            "coverage_full_gate_missing",
        ),
    ] {
        if value.pointer(ptr) != Some(&Value::Bool(true)) {
            out.push(code.to_string());
        }
    }
    if value.pointer("/fast_full_gate_split/fast_supports_completion") != Some(&Value::Bool(false))
    {
        out.push("coverage_fast_gate_used_for_completion".to_string());
    }
    for required in [
        "cli_tooling",
        "library_api",
        "branching_error_behavior",
        "product_ui_control_surface",
        "generated_authority",
        "workflow_orchestration",
        "security_trust_boundary",
        "install_cache_package",
    ] {
        if value
            .pointer(&format!("/behavior_dimension_mapping/{required}"))
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
        {
            out.push("coverage_behavior_dimension_missing".to_string());
        }
    }
    for target in strings(value, "required_target_paths") {
        if ["target", ".codex-worktree", "node_modules", "__pycache__"].contains(&target.as_str()) {
            out.push("coverage_target_path_ignored_local_state".to_string());
        }
    }
    out
}

fn scope_authority_failures(value: &Value, root: Option<&Path>) -> Vec<String> {
    let mut out = Vec::new();
    out.extend(builder_contract::manifest_dependency_failures(value));
    if let Some(root) = root {
        match digests::source_tree_digest(root, value) {
            Ok(actual) if str_field(value, "repo_root_digest") == actual => {}
            _ => out.push("coverage_receipt_source_digest_mismatch".to_string()),
        }
    } else if !str_field(value, "repo_root_digest").starts_with("sha256:")
        || str_field(value, "repo_root_digest") == crate::digest::ZERO
    {
        out.push("coverage_receipt_source_digest_mismatch".to_string());
    }
    let targets = strings(value, "required_target_paths");
    for root in roots::OWNED_ROOTS {
        if !targets.iter().any(|target| target == root) {
            out.push("coverage_manifest_target_gap".to_string());
        }
    }
    let owned = strings(value, "repo_owned_source_roots");
    for root in roots::OWNED_ROOTS {
        if !owned.iter().any(|item| item == root) {
            out.push("coverage_source_missing_from_manifest".to_string());
        }
    }
    if value.pointer("/changed_file_coupling_policy/required") != Some(&Value::Bool(true)) {
        out.push("coverage_changed_file_claim_not_withheld".to_string());
    }
    let policy = value
        .get("changed_file_coupling_policy")
        .unwrap_or(&Value::Null);
    out.extend(crate::audit::coverage::scope::changed_files::failures(
        root, policy,
    ));
    if let Some(root) = root {
        match digests::changed_files_digest(root, value) {
            Ok(actual) if str_field(policy, "changed_files_digest") == actual => {}
            _ => out.push("coverage_receipt_changed_files_digest_mismatch".to_string()),
        }
    } else if !str_field(policy, "changed_files_digest").starts_with("sha256:") {
        out.push("coverage_receipt_changed_files_digest_mismatch".to_string());
    }
    out.extend(dimensions::failures(value));
    out
}

fn str_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn strings(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect()
}
