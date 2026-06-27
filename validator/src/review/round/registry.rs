use crate::{digest, review::round::ReviewFailure};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

pub(crate) fn exposure_errors(root: &Path, receipt: &Value, out: &mut Vec<ReviewFailure>) {
    let Some(artifact) = receipt.get("live_registry_exposure") else {
        out.push(failure(
            "review_round_live_registry_exposure_missing",
            "receipt",
        ));
        return;
    };
    let Some(exposure) = read_exposure(root, artifact, out) else {
        return;
    };
    exposure_shape_errors(&exposure, out);
    exposure_identity_errors(receipt, &exposure, out);
    spawn_session_errors(receipt, &exposure, out);
    let exposed = exposed_agent_types(&exposure);
    for spec in crate::review::round::config::PERSONAS {
        if !exposed.contains(spec.agent_type) {
            out.push(failure(
                "review_round_live_registry_agent_missing",
                spec.persona,
            ));
        }
    }
}

fn exposure_identity_errors(receipt: &Value, exposure: &Value, out: &mut Vec<ReviewFailure>) {
    if exposure.get("source").and_then(Value::as_str) != Some("multi_agent_v1.tool_registry")
        || exposure.get("captured_at").and_then(Value::as_str)
            != receipt.get("generated_at").and_then(Value::as_str)
        || exposure
            .get("session_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .is_empty()
        || exposure.get("round_id").and_then(Value::as_str)
            != receipt.get("round_id").and_then(Value::as_str)
    {
        out.push(failure("review_round_live_registry_stale", "registry"));
    }
}

fn spawn_session_errors(receipt: &Value, exposure: &Value, out: &mut Vec<ReviewFailure>) {
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
                row.get("persona")
                    .and_then(Value::as_str)
                    .unwrap_or("reviewer"),
            ));
        }
    }
}

pub(crate) fn row_agent_type_error(row: &Value, persona: &str, out: &mut Vec<ReviewFailure>) {
    let Some(spec) = crate::review::round::config::persona_spec(persona) else {
        return;
    };
    if row.get("agent_type").and_then(Value::as_str) != Some(spec.agent_type) {
        out.push(failure(
            "review_round_live_registry_agent_mismatch",
            persona,
        ));
    }
}

fn read_exposure(root: &Path, artifact: &Value, out: &mut Vec<ReviewFailure>) -> Option<Value> {
    let rel = artifact.get("path").and_then(Value::as_str).unwrap_or("");
    if crate::package::inventory::package_path_error(root, rel).is_some() {
        out.push(failure("review_round_live_registry_artifact_invalid", rel));
        return None;
    }
    let path = root.join(rel);
    let want = artifact.get("digest").and_then(Value::as_str).unwrap_or("");
    let got = digest::file(&path).unwrap_or_else(|_| digest::ZERO.to_string());
    if got != want {
        out.push(failure("review_round_live_registry_artifact_mismatch", rel));
        return None;
    }
    match crate::json_boundary::read_json(&path) {
        Ok(value) => Some(value),
        Err(_) => {
            out.push(failure(
                "review_round_live_registry_artifact_malformed",
                rel,
            ));
            None
        }
    }
}

fn exposed_agent_types(value: &Value) -> BTreeSet<String> {
    value
        .get("agent_types")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|row| row.get("exposed").and_then(Value::as_bool) == Some(true))
        .filter_map(|row| row.get("agent_type").and_then(Value::as_str))
        .map(str::to_string)
        .collect()
}

fn exposure_shape_errors(value: &Value, out: &mut Vec<ReviewFailure>) {
    if value.get("schema").and_then(Value::as_str)
        != Some("harness-ultragoal.multi-agent-registry-exposure.v1")
    {
        out.push(failure(
            "review_round_live_registry_artifact_malformed",
            "schema",
        ));
    }
    for spec in crate::review::round::config::PERSONAS {
        let rows = value
            .get("agent_types")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|row| row.get("agent_type").and_then(Value::as_str) == Some(spec.agent_type))
            .collect::<Vec<_>>();
        if rows.len() != 1 {
            out.push(failure(
                "review_round_live_registry_agent_missing",
                spec.persona,
            ));
            continue;
        }
        let row = rows[0];
        if row.get("persona").and_then(Value::as_str) != Some(spec.persona)
            || row.get("custom_agent_path").and_then(Value::as_str) != Some(spec.custom_path)
        {
            out.push(failure(
                "review_round_live_registry_agent_mismatch",
                spec.persona,
            ));
        }
        if row.get("disk_cache_synced").and_then(Value::as_bool) != Some(true)
            || row.get("global_toml_present").and_then(Value::as_bool) != Some(true)
        {
            out.push(failure(
                "review_round_live_registry_disk_sync_missing",
                spec.persona,
            ));
        }
    }
}

fn failure(code: &str, detail: impl Into<String>) -> ReviewFailure {
    ReviewFailure::new("validator-execution-provenance", code, detail)
}
