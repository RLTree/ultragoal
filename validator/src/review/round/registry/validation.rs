use super::{RegistrySnapshot, failure};
use crate::review::round::ReviewFailure;
use serde_json::Value;

const MAX_REGISTRY_CAPTURE_AGE_SECONDS: i64 = 5 * 60;

pub(super) fn exposure_shape_errors(
    snapshot: &RegistrySnapshot,
    value: &Value,
    out: &mut Vec<ReviewFailure>,
) {
    for spec in crate::review::round::config::REVIEW_ROLES {
        let rows = value
            .get("agent_types")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|row| row.get("role").and_then(Value::as_str) == Some(spec.role_name))
            .collect::<Vec<_>>();
        if rows.len() != 1 {
            out.push(failure(
                "review_round_live_registry_agent_missing",
                spec.role_name,
            ));
            continue;
        }
        let row = rows[0];
        if row_has_retired_identity_or_inferred_runtime(row)
            || row.get("agent_manifest_path").and_then(Value::as_str)
                != Some(spec.agent_manifest_path)
            || row.get("agent_manifest_digest").and_then(Value::as_str)
                != snapshot.manifest_digest(spec.role_name, spec.agent_manifest_path)
            || row.get("source_manifest_present").and_then(Value::as_bool) != Some(true)
            || row.get("sandbox_mode").and_then(Value::as_str) != Some("read-only")
            || row.get("runtime_metadata_status").and_then(Value::as_str) != Some("unavailable")
            || row
                .get("custom_agent_discovery_status")
                .and_then(Value::as_str)
                != Some("unavailable")
            || row.get("exposed").and_then(Value::as_bool) != Some(false)
        {
            out.push(failure(
                "review_round_live_registry_agent_mismatch",
                spec.role_name,
            ));
        }
    }
}

pub(super) fn exposure_identity_errors(
    receipt: &Value,
    exposure: &Value,
    out: &mut Vec<ReviewFailure>,
) {
    if exposure.get("round_id").and_then(Value::as_str)
        != receipt.get("round_id").and_then(Value::as_str)
        || !registry_capture_is_current(receipt, exposure)
    {
        out.push(failure("review_round_live_registry_stale", "registry"));
    }
}

fn registry_capture_is_current(receipt: &Value, exposure: &Value) -> bool {
    let Some(round_generated) = receipt
        .get("generated_at")
        .and_then(Value::as_str)
        .and_then(crate::audit::clock::parse_iso_seconds)
    else {
        return false;
    };
    let Some(captured) = exposure
        .get("captured_at")
        .and_then(Value::as_str)
        .and_then(crate::audit::clock::parse_iso_seconds)
    else {
        return false;
    };
    captured <= round_generated && round_generated - captured <= MAX_REGISTRY_CAPTURE_AGE_SECONDS
}

pub(super) fn spawn_session_errors(
    receipt: &Value,
    exposure: &Value,
    out: &mut Vec<ReviewFailure>,
) {
    let session = exposure
        .get("session_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    for row in receipt
        .get("reviewers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if row
            .pointer("/live_spawn_receipt/source_thread_id")
            .and_then(Value::as_str)
            != Some(session)
        {
            out.push(failure(
                "review_round_live_registry_stale",
                "reviewer-session",
            ));
        }
    }
}

pub(super) fn row_agent_role_error(row: &Value, role: &str, out: &mut Vec<ReviewFailure>) {
    let Some(spec) = crate::review::round::config::review_role_spec(role) else {
        return;
    };
    if row.get("role").and_then(Value::as_str) != Some(spec.role_name) {
        out.push(failure(
            "review_round_live_registry_agent_mismatch",
            "reviewer-role",
        ));
    }
}

fn row_has_retired_identity_or_inferred_runtime(row: &Value) -> bool {
    [
        "agent_type",
        "persona",
        "custom_agent_path",
        "model",
        "reasoning",
        "model_reasoning_effort",
    ]
    .iter()
    .any(|key| row.get(*key).is_some())
}
