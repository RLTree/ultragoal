use super::super::command_failure::CommandFailureSummary;
use super::super::timing::NODE_TIMING_REL;
use super::full_command::FullCommandRun;
use super::observation::TelemetryReconciliation;
use super::observation_mode::ObservationMode;
use crate::cli::live_loop::{LiveLoopCommand, surfaces::LoopValidationSurface};
use serde_json::Value;
use std::path::Path;
use std::time::Instant;

mod row_fields;
mod telemetry_reuse;

use row_fields::{elapsed_ms, node_rows, text, valid_digest};

pub(super) struct CacheReplay {
    pub(super) run: FullCommandRun,
    pub(super) baseline: FullCommandRun,
    pub(super) prior_result_digest: String,
    pub(super) replayed_output_digest: String,
    pub(super) invalidation_proof: String,
    pub(super) telemetry_reconciliation: TelemetryReconciliation,
}

pub(super) fn verified_local_hit(
    root: &Path,
    surface: LoopValidationSurface,
    _candidate: &str,
    input_digest: &str,
    command: &LiveLoopCommand,
    cache_key: &str,
    started: Instant,
    observation_mode: ObservationMode,
) -> Option<CacheReplay> {
    if command.cache_mode != "verified-local" {
        return None;
    }
    let value = crate::json_boundary::read_json(&root.join(NODE_TIMING_REL)).ok()?;
    node_rows(&value).into_iter().find_map(|row| {
        replay_from_row(
            row,
            surface,
            input_digest,
            command,
            cache_key,
            started,
            observation_mode,
        )
    })
}

fn replay_from_row(
    row: &Value,
    surface: LoopValidationSurface,
    input_digest: &str,
    command: &LiveLoopCommand,
    cache_key: &str,
    started: Instant,
    observation_mode: ObservationMode,
) -> Option<CacheReplay> {
    if text(row, "node_id")? != surface.id
        || text(row, "tier")? != command.tier
        || text(row, "cache_mode")? != command.cache_mode
        || text(row, "input_digest")? != input_digest
        || text(row, "current_input_digest")? != input_digest
        || text(row, "canonical_full_command")? != surface.canonical_full_command
        || text(row, "cache_key")? != cache_key
        || text(row, "cache_honesty")? != "pass"
        || text(row, "validation_status")? != "pass"
        || text(row, "validation_cache_status")? != "reusable"
        || text(row, "validator_version")? != crate::cli::live_loop::graph::validator_version()
        || text(row, "law_version")? != crate::cli::live_loop::graph::law_version()
        || text(row, "schema_version")? != crate::cli::live_loop::graph::schema_version()
        || text(row, "fixture_version")? != crate::cli::live_loop::graph::fixture_version()
    {
        return None;
    }
    let proof_kind = text(row, "proof_kind")?;
    if !row_has_replayable_proof(row, proof_kind) {
        return None;
    }
    if !row_matches_observation_mode(row, observation_mode) {
        return None;
    }
    if !row_has_reconciled_duration(row) {
        return None;
    }
    let exit_code = row
        .get("exit_status")?
        .as_i64()
        .and_then(|value| i32::try_from(value).ok())?;
    let launch_error = row.get("verified_local_launch_error")?.as_bool()?;
    if launch_error || exit_code != 0 {
        return None;
    }
    let stdout_digest = valid_digest(text(row, "verified_local_stdout_digest")?)?;
    let stderr_digest = valid_digest(text(row, "verified_local_stderr_digest")?)?;
    let replayed_output_digest =
        crate::digest::bytes(format!("stdout={stdout_digest};stderr={stderr_digest}").as_bytes());
    if text(row, "output_digest")? != replayed_output_digest
        || text(row, "verified_local_output_digest")? != replayed_output_digest
    {
        return None;
    }
    let prior_result_digest = valid_digest(text(row, "result_digest")?)?;
    if text(row, "verified_local_result_digest")? != prior_result_digest {
        return None;
    }
    let cached_telemetry = telemetry_reuse::reconciliation(row)?;
    let baseline_duration_ms = row
        .get("baseline_duration_ms")?
        .as_u64()
        .filter(|value| *value > 0)?;
    let baseline_exit_code = row
        .get("baseline_exit_code")?
        .as_i64()
        .and_then(|value| i32::try_from(value).ok())?;
    let baseline_launch_error = row.get("baseline_launch_error")?.as_bool()?;
    let baseline_stdout_digest = valid_digest(text(row, "baseline_stdout_digest")?)?;
    let baseline_stderr_digest = valid_digest(text(row, "baseline_stderr_digest")?)?;
    Some(CacheReplay {
        run: FullCommandRun {
            exit_code,
            status_success: true,
            launch_error: false,
            duration_ms: elapsed_ms(started),
            stdout_digest: stdout_digest.to_string(),
            stderr_digest: stderr_digest.to_string(),
            failure: Default::default(),
        },
        baseline: FullCommandRun {
            exit_code: baseline_exit_code,
            status_success: !baseline_launch_error && baseline_exit_code == 0,
            launch_error: baseline_launch_error,
            duration_ms: baseline_duration_ms,
            stdout_digest: baseline_stdout_digest.to_string(),
            stderr_digest: baseline_stderr_digest.to_string(),
            failure: CommandFailureSummary::from_value(row.get("baseline_failure")),
        },
        prior_result_digest: prior_result_digest.to_string(),
        replayed_output_digest,
        invalidation_proof:
            "cache_key_current_input_digest_command_versions_and_environment_matched".to_string(),
        telemetry_reconciliation: cached_telemetry,
    })
}

fn row_matches_observation_mode(row: &Value, observation_mode: ObservationMode) -> bool {
    match observation_mode {
        ObservationMode::LoopRunSnapshot => true,
        ObservationMode::FullRoundtrip => text(row, "observability_status") == Some("pass"),
    }
}

fn row_has_replayable_proof(row: &Value, proof_kind: &str) -> bool {
    match proof_kind {
        "executed" => row_has_executed_work(row),
        "verified_cache_hit" => {
            row.get("cache_hit").and_then(Value::as_bool) == Some(true)
                && row.get("work_unit_count").and_then(Value::as_u64) == Some(0)
                && row_has_required_versions(row)
                && text(row, "equivalence_status") == Some("verified_same_candidate_cache_replay")
                && text(row, "cache_equivalence_status") == Some("pass")
                && text(row, "invalidation_proof").is_some_and(|value| !value.is_empty())
                && text(row, "prior_result_digest") == text(row, "result_digest")
                && text(row, "replayed_output_digest") == text(row, "output_digest")
        }
        _ => false,
    }
}

fn row_has_executed_work(row: &Value) -> bool {
    row.get("cache_hit").and_then(Value::as_bool) == Some(false)
        && row
            .get("work_unit_count")
            .and_then(Value::as_u64)
            .is_some_and(|value| value > 0)
        && row
            .get("actual_work_duration_ms")
            .and_then(Value::as_u64)
            .is_some_and(|value| value > 0)
        && row
            .get("graph_overhead_ms")
            .and_then(Value::as_u64)
            .is_some()
        && row_has_required_versions(row)
        && text(row, "equivalence_status") == Some("executed_current_candidate_not_cache_replay")
        && text(row, "invalidation_proof").is_some_and(|value| !value.is_empty())
        && row_has_command_argv(row)
}

fn row_has_reconciled_duration(row: &Value) -> bool {
    let actual = row.get("actual_work_duration_ms").and_then(Value::as_u64);
    let graph = row.get("graph_overhead_ms").and_then(Value::as_u64);
    let telemetry = row
        .get("telemetry_reconciliation_duration_ms")
        .and_then(Value::as_u64);
    let reconciled = row
        .get("reconciled_command_duration_ms")
        .and_then(Value::as_u64);
    let product_latency = row.get("product_latency_ms").and_then(Value::as_u64);
    match (actual, graph, telemetry, reconciled, product_latency) {
        (Some(actual), Some(graph), Some(telemetry), Some(reconciled), Some(product_latency)) => {
            reconciled == actual.saturating_add(graph).saturating_add(telemetry)
                && product_latency == actual.saturating_add(graph)
        }
        _ => false,
    }
}

fn row_has_required_versions(row: &Value) -> bool {
    [
        "validator_version",
        "law_version",
        "schema_version",
        "fixture_version",
    ]
    .into_iter()
    .all(|key| text(row, key).is_some_and(|value| !value.is_empty()))
}

fn row_has_command_argv(row: &Value) -> bool {
    row.get("command_argv")
        .and_then(Value::as_array)
        .is_some_and(|argv| {
            !argv.is_empty()
                && argv
                    .iter()
                    .all(|arg| arg.as_str().is_some_and(|value| !value.is_empty()))
        })
}

#[cfg(test)]
mod tests;
