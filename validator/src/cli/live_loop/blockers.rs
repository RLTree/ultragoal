use serde_json::{Value, json};

use super::graph;

pub(crate) fn status_for_blockers(product_blocker: &Value, first_blocker: &Value) -> &'static str {
    if product_blocker.get("id").and_then(Value::as_str) != Some("none") {
        return "fail";
    }
    if first_blocker.get("id").and_then(Value::as_str) == Some("none") {
        "pass"
    } else {
        "partial"
    }
}

pub(crate) fn first_product_blocker(nodes: &[Value]) -> Value {
    graph::first_product_blocker(nodes).unwrap_or_else(none)
}

pub(crate) fn first_control_board_blocker(current_state: &Value) -> Value {
    current_state
        .get("first_blocker")
        .filter(|blocker| blocker.get("id").and_then(Value::as_str) != Some("none"))
        .cloned()
        .unwrap_or_else(none)
}

pub(crate) fn first_loop_blocker(
    product: &Value,
    observability: &Value,
    speed: &Value,
    control_board: &Value,
) -> Value {
    [product, observability, speed, control_board]
        .into_iter()
        .find(|blocker| blocker.get("id").and_then(Value::as_str) != Some("none"))
        .cloned()
        .unwrap_or_else(none)
}

pub(crate) fn none() -> Value {
    json!({
        "id": "none",
        "surface": "none",
        "failure_class": "none",
        "where_failed": "none",
        "why_failed": "none",
        "next_repair": "none",
        "narrow_rerun": "none",
        "broad_rerun": "none",
        "claim_impact": "none"
    })
}
