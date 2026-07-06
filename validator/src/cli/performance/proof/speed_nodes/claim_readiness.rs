use super::{not_empty, timing_projection};
use serde_json::{Value, json};

pub(super) fn node_blocks_speed_claim(node: &Value) -> bool {
    !node_supports_positive_speed_claim(node)
}

fn node_supports_positive_speed_claim(node: &Value) -> bool {
    timing_projection::text(node, "timing_status") == Some("pass")
        && timing_projection::text(node, "failure_class") == Some("none")
        && timing_projection::text(node, "telemetry_reconciliation_status") == Some("pass")
        && timing_projection::text(node, "candidate_digest").is_some_and(not_empty)
        && timing_projection::text(node, "result_digest").is_some_and(not_empty)
        && timing_projection::text(node, "output_digest").is_some_and(not_empty)
        && timing_projection::text(node, "claim_impact").is_some_and(not_empty)
        && timing_projection::text(node, "where_failed").is_some_and(not_empty)
        && timing_projection::text(node, "why_failed").is_some_and(not_empty)
        && timing_projection::text(node, "next_repair").is_some_and(not_empty)
        && timing_projection::text(node, "claim_name").is_some_and(not_empty)
        && timing_projection::text(node, "product_behavior_observed").is_some_and(not_empty)
        && timing_projection::text(node, "proof_surface").is_some_and(not_empty)
        && timing_projection::text(node, "independent_reconciliation_surface")
            .is_some_and(not_empty)
        && node.get("cache_hit").and_then(Value::as_bool).is_some()
        && node
            .get("work_unit_count")
            .and_then(Value::as_u64)
            .is_some()
        && positive_number(node, "actual_work_duration_ms")
        && node
            .get("graph_overhead_ms")
            .and_then(Value::as_u64)
            .is_some()
        && match timing_projection::text(node, "proof_kind") {
            Some("executed") => node_has_executed_speed_proof(node),
            Some("verified_cache_hit") => node_has_cache_replay_speed_proof(node),
            _ => false,
        }
}

fn node_has_executed_speed_proof(node: &Value) -> bool {
    node.get("cache_hit").and_then(Value::as_bool) == Some(false)
        && node
            .get("work_unit_count")
            .and_then(Value::as_u64)
            .is_some_and(|value| value > 0)
        && nonempty_strings(node, "command_argv")
        && node.get("exit_status").and_then(Value::as_i64).is_some()
        && (nonempty_strings(node, "receipt_paths") || nonempty_strings(node, "artifact_paths"))
}

fn node_has_cache_replay_speed_proof(node: &Value) -> bool {
    node.get("cache_hit").and_then(Value::as_bool) == Some(true)
        && node.get("work_unit_count").and_then(Value::as_u64) == Some(0)
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
        .all(|key| timing_projection::text(node, key).is_some_and(not_empty))
        && timing_projection::text(node, "equivalence_status")
            == Some("verified_same_candidate_cache_replay")
        && timing_projection::text(node, "prior_result_digest")
            == timing_projection::text(node, "result_digest")
        && timing_projection::text(node, "replayed_output_digest")
            == timing_projection::text(node, "output_digest")
}

pub(super) fn first_blocker(nodes: &[Value]) -> Value {
    if nodes.is_empty() {
        return json!({
            "status": "blocked",
            "node_id": "missing_speed_node",
            "failure_class": "missing_current_node_speed_evidence",
            "where_failed": "performance.speed_proof.nodes",
            "why_failed": "speed_proof.nodes is empty; no executed command or verified-cache replay can support the speed claim",
            "next_repair": "run `ultragoal loop measure --node <id> --tier hot --cache-mode verified-local` for the blocker node"
        });
    };
    let Some(blocker) = nodes.iter().find(|node| node_blocks_speed_claim(node)) else {
        return Value::Null;
    };
    json!({
        "status": "blocked",
        "node_id": text(blocker, "node_id", "unknown_speed_node"),
        "failure_class": text(blocker, "failure_class", "unknown_failure_class"),
        "where_failed": text(blocker, "where_failed", "unknown_speed_node_path"),
        "why_failed": text(blocker, "why_failed", "speed proof row is not pass"),
        "next_repair": text(blocker, "next_repair", "rerun the narrow live-loop node measurement and repair the named node blocker")
    })
}

fn positive_number(node: &Value, key: &str) -> bool {
    node.get(key)
        .and_then(Value::as_u64)
        .is_some_and(|value| value > 0)
}

fn nonempty_strings(node: &Value, key: &str) -> bool {
    timing_projection::string_array(node, key)
        .is_some_and(|argv| !argv.is_empty() && argv.iter().all(|arg| !arg.is_empty()))
}

fn text<'a>(node: &'a Value, key: &str, fallback: &'a str) -> &'a str {
    timing_projection::text(node, key).unwrap_or(fallback)
}
