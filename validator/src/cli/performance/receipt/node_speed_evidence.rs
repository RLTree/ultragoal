use serde_json::Value;

pub(super) fn speed_claim_ready(value: &Value, expected_candidate: Option<&str>) -> bool {
    speed_failures(value, expected_candidate).is_empty()
}

pub(super) fn speed_failures(value: &Value, expected_candidate: Option<&str>) -> Vec<String> {
    let mut out = Vec::new();
    let Some(nodes) = value
        .pointer("/speed_proof/nodes")
        .and_then(Value::as_array)
    else {
        out.push("cli_performance_receipt_missing_node_speed_proof".to_string());
        return out;
    };
    if nodes.is_empty() {
        out.push("cli_performance_receipt_missing_node_speed_proof".to_string());
        return out;
    }
    for node in nodes {
        out.extend(speed_node_failures(node, expected_candidate));
    }
    out
}

fn speed_node_failures(node: &Value, expected_candidate: Option<&str>) -> Vec<String> {
    let mut out = Vec::new();
    let node_id = str_field(node, "node_id").unwrap_or("unknown_speed_node");
    let proof_kind = str_field(node, "proof_kind").unwrap_or("");
    for field in [
        "node_id",
        "proof_kind",
        "candidate_digest",
        "actual_work_duration_ms",
        "graph_overhead_ms",
        "result_digest",
        "output_digest",
        "telemetry_reconciliation_status",
        "claim_impact",
    ] {
        if node.get(field).is_none() {
            out.push(format!(
                "cli_performance_speed_node_missing:{node_id}:/{field}"
            ));
        }
    }
    if let Some(expected) = expected_candidate {
        let candidate = str_field(node, "candidate_digest").unwrap_or("");
        if candidate != expected {
            out.push(format!(
                "cli_performance_speed_node_candidate_digest_mismatch:{node_id}:{candidate}!={expected}"
            ));
        }
    }
    for field in ["candidate_digest", "result_digest", "output_digest"] {
        if let Some(value) = str_field(node, field) {
            if !valid_digest(value) {
                out.push(format!(
                    "cli_performance_speed_node_invalid_digest:{node_id}:/{field}"
                ));
            }
        }
    }
    if node
        .get("actual_work_duration_ms")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        == 0
    {
        out.push(format!(
            "cli_performance_speed_node_timing_proxy_only:{node_id}"
        ));
    }
    if node
        .get("graph_overhead_ms")
        .and_then(Value::as_u64)
        .is_none()
    {
        out.push(format!(
            "cli_performance_speed_node_missing:{node_id}:/graph_overhead_ms"
        ));
    }
    if str_field(node, "telemetry_reconciliation_status") != Some("pass") {
        out.push(format!(
            "cli_performance_speed_node_telemetry_not_reconciled:{node_id}"
        ));
    }
    match proof_kind {
        "executed" => executed_node_failures(node, node_id, &mut out),
        "verified_cache_hit" => verified_cache_node_failures(node, node_id, &mut out),
        _ => out.push(format!(
            "cli_performance_speed_node_invalid_proof_kind:{node_id}:{proof_kind}"
        )),
    }
    out
}

fn executed_node_failures(node: &Value, node_id: &str, out: &mut Vec<String>) {
    if node.get("cache_hit").and_then(Value::as_bool) != Some(false) {
        out.push(format!(
            "cli_performance_speed_node_cache_hit_without_verified_proof:{node_id}"
        ));
    }
    if node
        .get("work_unit_count")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        == 0
    {
        out.push(format!(
            "cli_performance_speed_node_zero_work_without_cache_equivalence:{node_id}"
        ));
    }
    if !nonempty_string_array(node, "command_argv") {
        out.push(format!(
            "cli_performance_speed_node_missing:{node_id}:/command_argv"
        ));
    }
    if node.get("exit_status").and_then(Value::as_i64).is_none() {
        out.push(format!(
            "cli_performance_speed_node_missing:{node_id}:/exit_status"
        ));
    }
    if !nonempty_string_array(node, "receipt_paths")
        && !nonempty_string_array(node, "artifact_paths")
    {
        out.push(format!(
            "cli_performance_speed_node_missing:{node_id}:/receipt_paths"
        ));
    }
}

fn verified_cache_node_failures(node: &Value, node_id: &str, out: &mut Vec<String>) {
    if node.get("cache_hit").and_then(Value::as_bool) != Some(true) {
        out.push(format!(
            "cli_performance_speed_node_cache_hit_missing:{node_id}"
        ));
    }
    if node
        .get("work_unit_count")
        .and_then(Value::as_u64)
        .unwrap_or(u64::MAX)
        != 0
    {
        out.push(format!(
            "cli_performance_speed_node_cache_replay_has_work_units:{node_id}"
        ));
    }
    for field in [
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
        if str_field(node, field).unwrap_or("").is_empty() {
            out.push(format!(
                "cli_performance_speed_node_missing:{node_id}:/{field}"
            ));
        }
    }
    for field in [
        "cache_key",
        "current_input_digest",
        "prior_result_digest",
        "replayed_output_digest",
    ] {
        if let Some(value) = str_field(node, field) {
            if !valid_digest(value) {
                out.push(format!(
                    "cli_performance_speed_node_invalid_digest:{node_id}:/{field}"
                ));
            }
        }
    }
    if str_field(node, "prior_result_digest") != str_field(node, "result_digest")
        || str_field(node, "replayed_output_digest") != str_field(node, "output_digest")
        || str_field(node, "equivalence_status") != Some("verified_same_candidate_cache_replay")
    {
        out.push(format!(
            "cli_performance_speed_node_unverified_cache_reuse:{node_id}"
        ));
    }
}

fn str_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn valid_digest(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn nonempty_string_array(value: &Value, key: &str) -> bool {
    value
        .get(key)
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty() && items.iter().all(|item| item.as_str().is_some()))
}
