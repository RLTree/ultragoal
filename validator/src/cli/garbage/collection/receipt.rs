use crate::cli::garbage::collection::operation::GC_RECEIPT_SCHEMA;
use serde_json::Value;

pub(crate) fn surface_value_failures(value: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if value.get("schema").and_then(Value::as_str) != Some(GC_RECEIPT_SCHEMA) {
        out.push("workspace_gc_receipt_wrong_schema".to_string());
    }
    for ptr in [
        "/issuer/tool",
        "/command/name",
        "/law_ids",
        "/digests/candidate",
        "/artifact_classification/classes",
        "/protected_set/protected_artifacts",
        "/deletion_plan/plan_digest",
        "/deletion_plan/plan_digest_source",
        "/deletion_plan/plan_digest_argument_required",
        "/post_verify/protected_artifacts_preserved",
        "/post_verify/apply_receipt_digest_required",
        "/post_verify/apply_receipt_digest",
        "/claim_ceiling",
    ] {
        if value.pointer(ptr).is_none() {
            out.push(format!("workspace_gc_receipt_missing:{ptr}"));
        }
    }
    let law_found = value
        .get("law_ids")
        .and_then(Value::as_array)
        .is_some_and(|rows| {
            rows.iter()
                .any(|row| row.as_str() == Some("workspace-artifact-cache-garbage-collection"))
        });
    if !law_found {
        out.push("workspace_gc_receipt_missing_law_id".to_string());
    }
    if value.get("status").and_then(Value::as_str) == Some("pass") {
        pass_binding_failures(value, &mut out);
    }
    out
}

fn pass_binding_failures(value: &Value, out: &mut Vec<String>) {
    if value.get("claim_ceiling").and_then(Value::as_str) != Some("gc_observation_bound") {
        out.push("workspace_gc_receipt_pass_claim_ceiling_not_gc_bound".to_string());
    }
    if value
        .get("observation_failures")
        .and_then(Value::as_array)
        .is_some_and(|failures| !failures.is_empty())
    {
        out.push("workspace_gc_receipt_pass_with_observation_failures".to_string());
    }
    let command = value
        .pointer("/command/name")
        .and_then(Value::as_str)
        .unwrap_or("");
    let plan_digest = value
        .pointer("/deletion_plan/plan_digest")
        .and_then(Value::as_str)
        .unwrap_or("");
    if plan_digest == crate::digest::ZERO || !plan_digest.starts_with("sha256:") {
        out.push("workspace_gc_receipt_plan_digest_not_bound".to_string());
    }
    if matches!(command, "dry_run" | "apply" | "verify") {
        if value
            .pointer("/deletion_plan/plan_digest_source")
            .and_then(Value::as_str)
            != Some("required_cli_argument")
        {
            out.push("workspace_gc_receipt_plan_digest_not_from_required_argument".to_string());
        }
        if value
            .pointer("/deletion_plan/plan_digest_argument_required")
            .and_then(Value::as_bool)
            != Some(true)
        {
            out.push("workspace_gc_receipt_plan_digest_argument_not_required".to_string());
        }
    }
    if command == "verify" {
        let apply = value
            .pointer("/post_verify/apply_receipt_digest")
            .and_then(Value::as_str)
            .unwrap_or("");
        if apply == crate::digest::ZERO || !apply.starts_with("sha256:") {
            out.push("workspace_gc_receipt_apply_digest_not_bound".to_string());
        }
        if value
            .pointer("/post_verify/apply_receipt_digest_required")
            .and_then(Value::as_bool)
            != Some(true)
        {
            out.push("workspace_gc_receipt_apply_digest_not_required".to_string());
        }
    }
}
