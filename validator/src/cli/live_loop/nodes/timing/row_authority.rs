use super::record_fields;
use serde_json::Value;
use std::path::{Component, Path};

pub(crate) const SOURCE_LOCAL_CLAIM_CEILING: &str = concat!(
    "source-local loop timing only; ",
    "readiness release completion final-packet and update_goal remain blocked"
);
const SOURCE_LOCAL_CLAIM_IMPACT: &str =
    "supports_live_loop_node_timing_only_no_readiness_release_completion_update_goal";

pub(crate) struct VerifiedLocalDigests {
    pub(crate) result_digest: String,
    pub(crate) output_digest: String,
    pub(crate) verified_local_result_digest: String,
    pub(crate) verified_local_output_digest: String,
}

pub(crate) fn verified_local_digests(root: &Path, row: &Value) -> Option<VerifiedLocalDigests> {
    let stdout_digest =
        record_fields::valid_digest(record_fields::text(row, "verified_local_stdout_digest")?)?;
    let stderr_digest =
        record_fields::valid_digest(record_fields::text(row, "verified_local_stderr_digest")?)?;
    let output_digest = record_fields::valid_digest(record_fields::text(row, "output_digest")?)?;
    let verified_local_output_digest =
        record_fields::valid_digest(record_fields::text(row, "verified_local_output_digest")?)?;
    let result_digest = record_fields::valid_digest(record_fields::text(row, "result_digest")?)?;
    let verified_local_result_digest =
        record_fields::valid_digest(record_fields::text(row, "verified_local_result_digest")?)?;
    let expected_output_digest = expected_output_digest(stdout_digest, stderr_digest);
    let expected_result_digest = expected_result_digest(row, &expected_output_digest)?;
    let authority = command_result_authority(root, row)?;
    (output_digest == expected_output_digest
        && verified_local_output_digest == expected_output_digest
        && result_digest == expected_result_digest
        && verified_local_result_digest == expected_result_digest
        && authority_matches_row(row, &authority, stdout_digest, stderr_digest))
    .then(|| VerifiedLocalDigests {
        result_digest: result_digest.to_string(),
        output_digest: output_digest.to_string(),
        verified_local_result_digest: verified_local_result_digest.to_string(),
        verified_local_output_digest: verified_local_output_digest.to_string(),
    })
}

pub(crate) fn proof_kind_is_claim_safe(row: &Value, digests: &VerifiedLocalDigests) -> bool {
    match record_fields::text(row, "proof_kind") {
        Some("executed") => {
            row.get("cache_hit").and_then(Value::as_bool) == Some(false)
                && row
                    .get("work_unit_count")
                    .and_then(Value::as_u64)
                    .is_some_and(|count| count > 0)
                && record_fields::text(row, "equivalence_status")
                    == Some("executed_current_candidate_not_cache_replay")
                && record_fields::nonempty_text(row, "invalidation_proof").is_some()
        }
        Some("verified_cache_hit") => {
            row.get("cache_hit").and_then(Value::as_bool) == Some(true)
                && row.get("work_unit_count").and_then(Value::as_u64) == Some(0)
                && record_fields::text(row, "prior_result_digest")
                    == Some(digests.result_digest.as_str())
                && record_fields::text(row, "replayed_output_digest")
                    == Some(digests.output_digest.as_str())
                && record_fields::text(row, "cache_equivalence_status") == Some("pass")
                && record_fields::text(row, "equivalence_status")
                    == Some("verified_same_candidate_cache_replay")
                && record_fields::nonempty_text(row, "invalidation_proof").is_some()
        }
        _ => false,
    }
}

pub(crate) fn rust_test_count_is_claim_safe(row: &Value, node_id: &str) -> bool {
    if node_id != "live_loop_measurement_rust_tests" {
        return true;
    }
    row.get("verified_local_executed_test_count")
        .and_then(Value::as_u64)
        .is_some_and(|count| count > 0)
}

pub(crate) fn claim_ceiling_is_source_local(row: &Value) -> bool {
    record_fields::text(row, "claim_ceiling") == Some(SOURCE_LOCAL_CLAIM_CEILING)
        && record_fields::text(row, "claim_impact") == Some(SOURCE_LOCAL_CLAIM_IMPACT)
}

fn expected_output_digest(stdout_digest: &str, stderr_digest: &str) -> String {
    crate::digest::bytes(format!("stdout={stdout_digest};stderr={stderr_digest}").as_bytes())
}

fn expected_result_digest(row: &Value, output_digest: &str) -> Option<String> {
    let exit_code = record_fields::i32_field(row, "exit_status")?;
    let launch_error = row.get("verified_local_launch_error")?.as_bool()?;
    Some(crate::digest::bytes(
        format!("exit={exit_code};launch={launch_error};output={output_digest}").as_bytes(),
    ))
}

fn command_result_authority(root: &Path, row: &Value) -> Option<Value> {
    let rel = first_text_field(
        row.get("telemetry_reconciliation")?,
        "command_observation_receipt",
    )?;
    let path = safe_observation_receipt_path(root, rel)?;
    let receipt = crate::json_boundary::read_json(&path).ok()?;
    if !command_observation_matches_row(row, rel, &receipt) {
        return None;
    }
    receipt.get("command_result_authority").cloned()
}

fn safe_observation_receipt_path(root: &Path, rel: &str) -> Option<std::path::PathBuf> {
    let rel_path = Path::new(rel);
    if rel_path.is_absolute()
        || !rel.starts_with("validation_artifacts/observability/live-loop/commands/")
        || !rel.ends_with("-command-observation.json")
        || rel_path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        return None;
    }
    Some(root.join(rel_path))
}

fn first_text_field<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    if let Some(text) = record_fields::text(value, field) {
        return Some(text);
    }
    match value {
        Value::Array(items) => items.iter().find_map(|item| first_text_field(item, field)),
        Value::Object(object) => object
            .values()
            .find_map(|item| first_text_field(item, field)),
        _ => None,
    }
}

fn authority_matches_row(
    row: &Value,
    authority: &Value,
    stdout_digest: &str,
    stderr_digest: &str,
) -> bool {
    record_fields::text(authority, "authority") == Some("live_loop_command_observation_actual_work")
        && record_fields::text(authority, "stdout_digest") == Some(stdout_digest)
        && record_fields::text(authority, "stderr_digest") == Some(stderr_digest)
        && record_fields::text(authority, "output_digest")
            == record_fields::text(row, "output_digest")
        && record_fields::text(authority, "result_digest")
            == record_fields::text(row, "result_digest")
        && authority.get("exit_status").and_then(Value::as_i64)
            == row.get("exit_status").and_then(Value::as_i64)
        && authority.get("launch_error").and_then(Value::as_bool)
            == row
                .get("verified_local_launch_error")
                .and_then(Value::as_bool)
}

fn command_observation_matches_row(row: &Value, rel: &str, receipt: &Value) -> bool {
    let Some(node_id) = record_fields::text(row, "node_id") else {
        return false;
    };
    let Some(candidate) = record_fields::text(row, "candidate_digest") else {
        return false;
    };
    let operation = format!("loop.measure.{node_id}");
    record_fields::text(receipt, "status") == Some("pass")
        && record_fields::text(receipt, "candidate_digest") == Some(candidate)
        && record_fields::text(receipt, "receipt_path") == Some(rel)
        && record_fields::text(receipt, "operation") == Some(operation.as_str())
        && receipt
            .get("event")
            .is_some_and(|event| command_observation_event_matches_row(row, node_id, event))
}

fn command_observation_event_matches_row(row: &Value, node_id: &str, event: &Value) -> bool {
    let argv = command_argv(row);
    let Some(command) = argv.first().map(String::as_str) else {
        return false;
    };
    let subcommand = argv[1..].join(" ");
    let operation = format!("loop.measure.{node_id}");
    record_fields::text(event, "operation") == Some(operation.as_str())
        && record_fields::text(event, "status") == Some("pass")
        && record_fields::text(event, "candidate_digest")
            == record_fields::text(row, "candidate_digest")
        && record_fields::text(event, "artifact_path") == Some(super::NODE_TIMING_REL)
        && record_fields::text(event, "command") == Some(command)
        && record_fields::text(event, "subcommand") == Some(subcommand.as_str())
}

fn command_argv(row: &Value) -> Vec<String> {
    row.get("command_argv")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(ToString::to_string))
                .collect()
        })
        .unwrap_or_default()
}
