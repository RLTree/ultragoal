use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::Path;

const NODE_TIMING_REL: &str = "validation_artifacts/observability/live-loop-node-timing.json";

#[derive(Clone, Copy)]
enum SpeedProofKind {
    Executed,
    VerifiedCacheHit,
}

impl SpeedProofKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Executed => "executed",
            Self::VerifiedCacheHit => "verified_cache_hit",
        }
    }
}

pub(super) fn speed_proof_value(root: &Path, candidate: &str, enabled: bool) -> Value {
    let nodes = if enabled {
        current_pass_nodes(root, candidate)
    } else {
        Vec::new()
    };
    json!({
        "status": if nodes.is_empty() { "blocked" } else { "pass" },
        "nodes": nodes,
        "proof_boundary": "wrapper_latency_is_observation_only_until_node_work_is_bound",
        "required_node_proof": "executed_or_verified_same_candidate_cache_replay",
        "claim_impact": "performance_claims_withheld_until_speed_nodes_record_product_work_or_verified_reuse"
    })
}

fn current_pass_nodes(root: &Path, candidate: &str) -> Vec<Value> {
    let timing = crate::json_boundary::read_json(&root.join(NODE_TIMING_REL))
        .unwrap_or_else(|_| json!({"nodes":[]}));
    let mut nodes = timing
        .get("nodes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|row| text(row, "candidate_digest") == Some(candidate))
        .filter(|row| text(row, "timing_status") == Some("pass"))
        .filter(|row| text(row, "failure_class") == Some("none"))
        .filter_map(project_node)
        .collect::<Vec<_>>();
    nodes.sort_by(|left, right| text(left, "node_id").cmp(&text(right, "node_id")));
    nodes
}

fn project_node(row: &Value) -> Option<Value> {
    let proof_kind = speed_proof_kind(row)?;
    if !row_supports_speed_claim(row, proof_kind) {
        return None;
    }
    let receipt_paths = receipt_paths(row);
    let mut node = json!({
        "node_id": text(row, "node_id")?,
        "proof_kind": proof_kind.as_str(),
        "candidate_digest": text(row, "candidate_digest")?,
        "cache_hit": row.get("cache_hit")?.as_bool()?,
        "work_unit_count": row.get("work_unit_count")?.as_u64()?,
        "actual_work_duration_ms": row.get("actual_work_duration_ms")?.as_u64()?,
        "graph_overhead_ms": row.get("graph_overhead_ms")?.as_u64()?,
        "result_digest": text(row, "result_digest")?,
        "output_digest": text(row, "output_digest")?,
        "telemetry_reconciliation_status": text(row, "telemetry_reconciliation_status")?,
        "claim_impact": text(row, "claim_impact")?,
        "command_argv": string_array(row, "verified_local_command_argv")?,
        "exit_status": row.get("verified_local_exit_code")?.as_i64()?,
        "receipt_paths": receipt_paths,
        "artifact_paths": [NODE_TIMING_REL],
    });
    match proof_kind {
        SpeedProofKind::Executed => Some(node),
        SpeedProofKind::VerifiedCacheHit => {
            let object = node
                .as_object_mut()
                .expect("projected speed proof node is always an object");
            for key in [
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
            ] {
                let value = text(row, key).expect("verified cache proof fields are prevalidated");
                object.insert(key.to_string(), json!(value));
            }
            Some(node)
        }
    }
}

fn speed_proof_kind(row: &Value) -> Option<SpeedProofKind> {
    match text(row, "proof_kind")? {
        "executed" => Some(SpeedProofKind::Executed),
        "verified_cache_hit" => Some(SpeedProofKind::VerifiedCacheHit),
        _ => None,
    }
}

fn row_supports_speed_claim(row: &Value, proof_kind: SpeedProofKind) -> bool {
    row_has_claim_bearing_speed_fields(row)
        && match proof_kind {
            SpeedProofKind::Executed => row_has_executed_speed_proof(row),
            SpeedProofKind::VerifiedCacheHit => row_has_cache_replay_speed_proof(row),
        }
}

fn row_has_claim_bearing_speed_fields(row: &Value) -> bool {
    text(row, "telemetry_reconciliation_status") == Some("pass")
        && text(row, "candidate_digest").is_some_and(not_empty)
        && text(row, "result_digest").is_some_and(not_empty)
        && text(row, "output_digest").is_some_and(not_empty)
        && text(row, "claim_impact").is_some_and(not_empty)
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
        && string_array(row, "verified_local_command_argv")
            .is_some_and(|argv| !argv.is_empty() && argv.iter().all(|arg| !arg.is_empty()))
        && row
            .get("verified_local_exit_code")
            .and_then(Value::as_i64)
            .is_some()
        && !receipt_paths(row).is_empty()
}

fn row_has_executed_speed_proof(row: &Value) -> bool {
    row.get("cache_hit").and_then(Value::as_bool) == Some(false)
        && row
            .get("work_unit_count")
            .and_then(Value::as_u64)
            .is_some_and(|value| value > 0)
}

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
        .all(|key| text(row, key).is_some_and(not_empty))
        && text(row, "cache_equivalence_status") == Some("pass")
        && text(row, "equivalence_status") == Some("verified_same_candidate_cache_replay")
        && text(row, "prior_result_digest") == text(row, "result_digest")
        && text(row, "replayed_output_digest") == text(row, "output_digest")
}

fn receipt_paths(row: &Value) -> Vec<String> {
    let mut paths = BTreeSet::new();
    if let Some(path) = text(row, "receipt_path") {
        paths.insert(path.to_string());
    }
    if let Some(path) = row
        .pointer("/verified_local_failure/receipt")
        .and_then(Value::as_str)
        .filter(|path| !path.is_empty())
    {
        paths.insert(path.to_string());
    }
    paths.into_iter().collect()
}

fn string_array(row: &Value, key: &str) -> Option<Vec<String>> {
    row.get(key)?
        .as_array()?
        .iter()
        .map(|item| item.as_str().map(str::to_string))
        .collect()
}

fn text<'a>(row: &'a Value, key: &str) -> Option<&'a str> {
    row.get(key).and_then(Value::as_str)
}

fn not_empty(value: &str) -> bool {
    !value.is_empty()
}

#[cfg(test)]
mod tests;
