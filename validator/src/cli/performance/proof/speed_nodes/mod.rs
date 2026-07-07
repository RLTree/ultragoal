mod claim_readiness;
mod timing_projection;

use serde_json::{Value, json};
use std::path::Path;

pub(super) const NODE_TIMING_REL: &str =
    "validation_artifacts/observability/live-loop-node-timing.json";

#[cfg(test)]
#[derive(Clone, Copy)]
enum SpeedProofKind {
    Executed,
    VerifiedCacheHit,
}

pub(super) fn speed_proof_value(root: &Path, candidate: &str, enabled: bool) -> Value {
    let nodes = if enabled {
        timing_projection::current_nodes(root, candidate)
    } else {
        Vec::new()
    };
    let status = if nodes.is_empty() || nodes.iter().any(claim_readiness::node_blocks_speed_claim) {
        "blocked"
    } else {
        "pass"
    };
    json!({
        "status": status,
        "nodes": nodes,
        "first_blocker": claim_readiness::first_blocker(&nodes),
        "proof_boundary": "wrapper_latency_is_observation_only_until_node_work_is_bound",
        "required_node_proof": "executed_or_verified_same_candidate_cache_replay",
        "claim_impact": "performance_claims_withheld_until_speed_nodes_record_product_work_or_verified_reuse"
    })
}

#[cfg(test)]
fn speed_proof_kind(row: &Value) -> Option<SpeedProofKind> {
    match timing_projection::text(row, "proof_kind")? {
        "executed" => Some(SpeedProofKind::Executed),
        "verified_cache_hit" => Some(SpeedProofKind::VerifiedCacheHit),
        _ => None,
    }
}

#[cfg(test)]
fn row_supports_speed_claim(row: &Value, proof_kind: SpeedProofKind) -> bool {
    row_has_claim_bearing_speed_fields(row)
        && match proof_kind {
            SpeedProofKind::Executed => row_has_executed_speed_proof(row),
            SpeedProofKind::VerifiedCacheHit => row_has_cache_replay_speed_proof(row),
        }
}

#[cfg(test)]
fn row_has_claim_bearing_speed_fields(row: &Value) -> bool {
    timing_projection::text(row, "telemetry_reconciliation_status") == Some("pass")
        && timing_projection::text(row, "candidate_digest").is_some_and(not_empty)
        && row_has_currentness_fields(row)
        && timing_projection::text(row, "result_digest").is_some_and(not_empty)
        && timing_projection::text(row, "output_digest").is_some_and(not_empty)
        && timing_projection::text(row, "claim_impact").is_some_and(not_empty)
        && row.get("cache_hit").and_then(Value::as_bool).is_some()
        && row.get("work_unit_count").and_then(Value::as_u64).is_some()
        && row
            .get("actual_work_duration_ms")
            .and_then(Value::as_u64)
            .is_some_and(|value| value > 0)
        && row
            .get("graph_overhead_ms")
            .and_then(Value::as_u64)
            .is_some()
        && row_has_reconciled_product_latency(row)
        && timing_projection::string_array(row, "command_argv")
            .is_some_and(|argv| !argv.is_empty() && argv.iter().all(|arg| !arg.is_empty()))
        && row.get("exit_status").and_then(Value::as_i64).is_some()
        && (timing_projection::string_array(row, "receipt_paths")
            .is_some_and(|paths| !paths.is_empty() && paths.iter().all(|path| !path.is_empty()))
            || timing_projection::string_array(row, "artifact_paths").is_some_and(|paths| {
                !paths.is_empty() && paths.iter().all(|path| !path.is_empty())
            }))
}

#[cfg(test)]
fn row_has_currentness_fields(row: &Value) -> bool {
    [
        "tier",
        "cache_mode",
        "input_digest",
        "current_input_digest",
        "audit_context_digest",
        "cache_key",
        "validator_version",
        "law_version",
        "schema_version",
        "fixture_version",
    ]
    .into_iter()
    .all(|key| timing_projection::text(row, key).is_some_and(not_empty))
}

#[cfg(test)]
pub(super) fn row_has_reconciled_product_latency(row: &Value) -> bool {
    let Some(actual) = row.get("actual_work_duration_ms").and_then(Value::as_u64) else {
        return false;
    };
    let Some(graph) = row.get("graph_overhead_ms").and_then(Value::as_u64) else {
        return false;
    };
    let Some(telemetry) = row
        .get("telemetry_reconciliation_duration_ms")
        .and_then(Value::as_u64)
    else {
        return false;
    };
    let Some(reconciled) = row
        .get("reconciled_command_duration_ms")
        .and_then(Value::as_u64)
    else {
        return false;
    };
    let Some(product_latency) = row.get("product_latency_ms").and_then(Value::as_u64) else {
        return false;
    };
    reconciled == actual.saturating_add(graph).saturating_add(telemetry)
        && product_latency == actual.saturating_add(graph)
}

#[cfg(test)]
fn row_has_executed_speed_proof(row: &Value) -> bool {
    row.get("cache_hit").and_then(Value::as_bool) == Some(false)
        && row
            .get("work_unit_count")
            .and_then(Value::as_u64)
            .is_some_and(|value| value > 0)
}

#[cfg(test)]
fn row_has_cache_replay_speed_proof(row: &Value) -> bool {
    row.get("cache_hit").and_then(Value::as_bool) == Some(true)
        && row.get("work_unit_count").and_then(Value::as_u64) == Some(0)
        && [
            "cache_key",
            "current_input_digest",
            "validator_version",
            "law_version",
            "schema_version",
            "fixture_version",
            "prior_result_digest",
            "replayed_output_digest",
            "equivalence_status",
            "invalidation_proof",
        ]
        .into_iter()
        .all(|key| timing_projection::text(row, key).is_some_and(not_empty))
        && timing_projection::text(row, "cache_equivalence_status") == Some("pass")
        && timing_projection::text(row, "equivalence_status")
            == Some("verified_same_candidate_cache_replay")
        && timing_projection::text(row, "prior_result_digest")
            == timing_projection::text(row, "result_digest")
        && timing_projection::text(row, "replayed_output_digest")
            == timing_projection::text(row, "output_digest")
}

pub(super) fn not_empty(value: &str) -> bool {
    !value.is_empty()
}

#[cfg(test)]
mod tests;
