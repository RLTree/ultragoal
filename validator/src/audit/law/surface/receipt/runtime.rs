use crate::audit::law::surface::receipt::requirements::{
    array_contains, artifact_digest_failures, require_nonempty, require_schema,
};
use serde_json::Value;
use std::path::Path;

const ROOT_SUFFIX: &str = "harness-ultragoal-plugin-proposal";

pub fn runtime_tool_identity_value_failures(root: &Path, value: &Value) -> Vec<String> {
    let mut out = Vec::new();
    require_schema(
        value,
        "harness-ultragoal.runtime-tool-identity-receipt.v1",
        "runtime_tool_identity_wrong_schema",
        &mut out,
    );
    let tool = value.get("tool_identity").unwrap_or(&Value::Null);
    require_nonempty(tool, "name", "runtime_tool_identity_missing_tool", &mut out);
    let version = require_nonempty(
        tool,
        "version",
        "runtime_tool_identity_missing_version",
        &mut out,
    );
    if matches!(version.as_deref(), Some("stale" | "unknown" | "0.0.0")) {
        out.push("runtime_tool_identity_stale_version".to_string());
    }
    require_nonempty(
        tool,
        "binary_path",
        "runtime_tool_identity_missing_binary_path",
        &mut out,
    );
    if !value
        .get("workspace")
        .and_then(Value::as_str)
        .is_some_and(|workspace| workspace.ends_with(ROOT_SUFFIX))
    {
        out.push("runtime_tool_identity_wrong_workspace".to_string());
    }
    artifact_digest_failures(
        root,
        value,
        "artifact_digests",
        "runtime_tool_identity_digest_mismatch",
        &mut out,
    );
    if value.get("claim_ceiling").and_then(Value::as_str) != Some("same_tool_runtime_surface_only")
    {
        out.push("runtime_tool_identity_claim_ceiling_not_same_surface".to_string());
    }
    out
}

pub fn product_live_surface_value_failures(root: &Path, value: &Value) -> Vec<String> {
    let mut out = Vec::new();
    require_schema(
        value,
        "harness-ultragoal.product-live-surface-receipt.v1",
        "product_live_surface_wrong_schema",
        &mut out,
    );
    artifact_digest_failures(
        root,
        value,
        "evidence",
        "product_live_surface_digest_mismatch",
        &mut out,
    );
    for required in [
        "cli_fixture_success",
        "install_success",
        "package_publication",
        "reviewer_agreement",
        "smoke_test",
    ] {
        if !array_contains(value, "substitutions_rejected", required) {
            out.push(format!(
                "product_live_surface_substitute_not_rejected:{required}"
            ));
        }
    }
    if value.get("claim_ceiling").and_then(Value::as_str) != Some("same_live_surface_only") {
        out.push("product_live_surface_claim_ceiling_not_same_surface".to_string());
    }
    out
}

pub fn transcript_quality_value_failures(_root: &Path, value: &Value) -> Vec<String> {
    let mut out = Vec::new();
    require_schema(
        value,
        "harness-ultragoal.transcript-quality-receipt.v1",
        "transcript_quality_wrong_schema",
        &mut out,
    );
    if value
        .get("post_stop_batch_transcription")
        .and_then(Value::as_str)
        != Some("complete")
    {
        out.push("transcript_quality_finalization_disabled".to_string());
    }
    if value.get("cleanup_complete").and_then(Value::as_bool) != Some(true) {
        out.push("transcript_quality_cleanup_incomplete".to_string());
    }
    if value
        .get("final_alignment_complete")
        .and_then(Value::as_bool)
        != Some(true)
    {
        out.push("transcript_quality_alignment_incomplete".to_string());
    }
    if value.get("claim_ceiling").and_then(Value::as_str) != Some("transcript_reuse_supported") {
        out.push("transcript_quality_claim_ceiling_not_supported".to_string());
    }
    out
}
