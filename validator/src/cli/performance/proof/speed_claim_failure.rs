use serde_json::Value;

pub(super) fn observed_value(value: &Value) -> String {
    let failures = super::super::receipt::speed_proof_failures(value, None);
    let Some(nodes) = value
        .pointer("/speed_proof/nodes")
        .and_then(Value::as_array)
    else {
        return format!("speed_proof.nodes missing; failures={failures:?}");
    };
    let Some(node) = nodes.first() else {
        return format!("speed_proof.nodes=[]; failures={failures:?}");
    };
    let node_id = text(node, "node_id").unwrap_or("unknown_speed_node");
    let proof_kind = text(node, "proof_kind").unwrap_or("missing");
    let timing_status = text(node, "timing_status").unwrap_or("missing");
    let failure_class = text(node, "failure_class").unwrap_or("missing");
    let where_failed = text(node, "where_failed").unwrap_or("missing");
    let why_failed = text(node, "why_failed").unwrap_or("missing");
    let next_repair = text(node, "next_repair").unwrap_or("missing");
    let actual = number(node, "actual_work_duration_ms");
    let work_units = number(node, "work_unit_count");
    let cache_hit = node
        .get("cache_hit")
        .and_then(Value::as_bool)
        .map_or_else(|| "missing".to_string(), |value| value.to_string());
    format!(
        "speed_node={node_id}; proof_kind={proof_kind}; timing_status={timing_status}; \
         failure_class={failure_class}; where_failed={where_failed}; \
         why_failed={why_failed}; next_repair={next_repair}; \
         actual_work_duration_ms={actual}; work_unit_count={work_units}; \
         cache_hit={cache_hit}; failures={failures:?}"
    )
}

fn number(node: &Value, key: &str) -> String {
    node.get(key)
        .and_then(Value::as_u64)
        .map_or_else(|| "missing".to_string(), |value| value.to_string())
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}
