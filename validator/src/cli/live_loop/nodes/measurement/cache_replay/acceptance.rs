use super::fields::text;
use crate::cli::live_loop::nodes::measurement::ObservationMode;
use serde_json::Value;

pub(in crate::cli::live_loop::nodes::measurement::cache_replay) fn matches_observation_mode(
    row: &Value,
    observation_mode: ObservationMode,
) -> bool {
    match observation_mode {
        ObservationMode::LoopRunSnapshot => true,
        ObservationMode::FullRoundtrip => text(row, "observability_status") == Some("pass"),
    }
}

pub(in crate::cli::live_loop::nodes::measurement::cache_replay) fn has_replayable_proof(
    row: &Value,
    proof_kind: &str,
) -> bool {
    match proof_kind {
        "executed" => has_executed_work(row),
        "verified_cache_hit" => {
            row.get("cache_hit").and_then(Value::as_bool) == Some(true)
                && row.get("work_unit_count").and_then(Value::as_u64) == Some(0)
                && has_required_versions(row)
                && text(row, "equivalence_status") == Some("verified_same_candidate_cache_replay")
                && text(row, "cache_equivalence_status") == Some("pass")
                && text(row, "invalidation_proof").is_some_and(|value| !value.is_empty())
                && text(row, "prior_result_digest") == text(row, "result_digest")
                && text(row, "replayed_output_digest") == text(row, "output_digest")
        }
        _ => false,
    }
}

fn has_executed_work(row: &Value) -> bool {
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
        && has_required_versions(row)
        && text(row, "equivalence_status") == Some("executed_current_candidate_not_cache_replay")
        && text(row, "invalidation_proof").is_some_and(|value| !value.is_empty())
        && has_command_argv(row)
}

pub(in crate::cli::live_loop::nodes::measurement::cache_replay) fn has_reconciled_duration(
    row: &Value,
) -> bool {
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

fn has_required_versions(row: &Value) -> bool {
    [
        "validator_version",
        "law_version",
        "schema_version",
        "fixture_version",
    ]
    .into_iter()
    .all(|key| text(row, key).is_some_and(|value| !value.is_empty()))
}

fn has_command_argv(row: &Value) -> bool {
    row.get("command_argv")
        .and_then(Value::as_array)
        .is_some_and(|argv| {
            !argv.is_empty()
                && argv
                    .iter()
                    .all(|arg| arg.as_str().is_some_and(|value| !value.is_empty()))
        })
}
