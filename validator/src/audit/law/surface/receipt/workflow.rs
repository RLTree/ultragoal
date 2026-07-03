use crate::audit::law::surface::receipt::requirements::{
    array_contains, artifact_digest_failures, require_nonempty, require_schema,
};
use serde_json::Value;
use std::path::Path;

pub fn clean_checkout_value_failures(root: &Path, value: &Value) -> Vec<String> {
    let mut out = Vec::new();
    require_schema(
        value,
        "harness-ultragoal.clean-checkout-command-discovery-receipt.v1",
        "clean_checkout_wrong_schema",
        &mut out,
    );
    require_nonempty(
        value,
        "root_check_command",
        "clean_checkout_missing_root_check",
        &mut out,
    );
    require_nonempty(
        value,
        "installed_check_command",
        "clean_checkout_missing_installed_command",
        &mut out,
    );
    if value
        .get("source_install_cache_command_alignment")
        .and_then(Value::as_str)
        != Some("same_candidate")
    {
        out.push("clean_checkout_source_install_cache_command_drift".to_string());
    }
    if value.get("claim_ceiling").and_then(Value::as_str)
        != Some("commands_discoverable_from_clean_checkout")
    {
        out.push("clean_checkout_claim_ceiling_not_discoverable".to_string());
    }
    command_failures(root, value, &mut out);
    out
}

pub fn restartable_execplan_value_failures(_root: &Path, value: &Value) -> Vec<String> {
    let mut out = Vec::new();
    require_schema(
        value,
        "harness-ultragoal.restartable-execplan-receipt.v1",
        "restartable_execplan_wrong_schema",
        &mut out,
    );
    let sections = value.get("required_sections").unwrap_or(&Value::Null);
    for section in [
        "purpose",
        "user_outcome",
        "steps",
        "current_state",
        "discoveries",
        "decisions",
        "validation_commands",
        "idempotence",
        "recovery",
        "working_behavior",
    ] {
        if sections.get(section).and_then(Value::as_bool) != Some(true) {
            out.push(format!("restartable_execplan_missing_section:{section}"));
        }
    }
    if value.get("stale").and_then(Value::as_bool) != Some(false) {
        out.push("restartable_execplan_stale".to_string());
    }
    if value.get("prose_only").and_then(Value::as_bool) != Some(false) {
        out.push("restartable_execplan_prose_only".to_string());
    }
    if value.get("claim_ceiling").and_then(Value::as_str) != Some("restartable_execplans_enforced")
    {
        out.push("restartable_execplan_claim_ceiling_not_enforced".to_string());
    }
    out
}

pub fn memory_context_value_failures(root: &Path, value: &Value) -> Vec<String> {
    let mut out = Vec::new();
    require_schema(
        value,
        "harness-ultragoal.memory-context-boundary-receipt.v1",
        "memory_context_wrong_schema",
        &mut out,
    );
    if value.get("memory_context_only").and_then(Value::as_bool) != Some(true) {
        out.push("memory_context_not_marked_context_only".to_string());
    }
    for required in ["memory_only", "summary_only"] {
        if !array_contains(value, "substitutions_rejected", required) {
            out.push(format!("memory_context_substitute_not_rejected:{required}"));
        }
    }
    artifact_digest_failures(
        root,
        value,
        "memory_artifacts",
        "memory_context_artifact_digest_mismatch",
        &mut out,
    );
    artifact_digest_failures(
        root,
        value,
        "live_same_surface_evidence",
        "memory_context_live_evidence_digest_mismatch",
        &mut out,
    );
    if value.get("claim_ceiling").and_then(Value::as_str)
        != Some("live_same_surface_evidence_required")
    {
        out.push("memory_context_claim_ceiling_not_same_surface".to_string());
    }
    out
}

fn command_failures(root: &Path, value: &Value, out: &mut Vec<String>) {
    let commands = value
        .get("commands")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if commands.is_empty() {
        out.push("clean_checkout_missing_commands".to_string());
    }
    for command in commands {
        let id = command
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        if command
            .get("command")
            .and_then(Value::as_str)
            .is_none_or(|text| {
                text.trim().is_empty() || text.contains("see docs") || text.contains("remember")
            })
        {
            out.push(format!("clean_checkout_prose_only_command:{id}"));
        }
        if command.get("status").and_then(Value::as_str) != Some("passed") {
            out.push(format!("clean_checkout_non_runnable_command:{id}"));
        }
        if command
            .get("requires_local_author_memory")
            .and_then(Value::as_bool)
            != Some(false)
        {
            out.push(format!("clean_checkout_local_state_dependency:{id}"));
        }
        artifact_digest_failures(
            root,
            &command,
            "artifacts",
            "clean_checkout_digest_mismatch",
            out,
        );
    }
}
