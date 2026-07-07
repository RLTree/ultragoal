use serde_json::{Value, json};

pub(in crate::cli::observe::explain) fn fallback_from_receipt(value: &Value) -> Value {
    let get = |key: &str| value.get(key).cloned().unwrap_or(Value::Null);
    let operation = value
        .get("operation")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    json!({
        "run_id": get("run_id"),
        "candidate_digest": get("candidate_digest"),
        "operation": get("operation"),
        "status": "fail",
        "failure_class": "receipt_without_observability_event",
        "why_failed": format!(
            "observability receipt for {operation} matched the target selector but has no event object"
        ),
        "where_failed": "observe.target.receipt_event_binding",
        "next_repair": format!(
            "rerun {operation} with real event emission, then query logs metrics traces by run/correlation/current digest"
        ),
        "claim_impact": "observability_reconciliation_blocked",
        "observed_receipt_status": get("status"),
        "fallback_only": true,
        "law_id": get("law_id"),
        "check_id": get("check_id"),
        "claim_id": get("claim_id"),
        "query_hint_logql": get("query_hint_logql"),
        "query_hint_promql": get("query_hint_promql"),
        "query_hint_traceql": get("query_hint_traceql")
    })
}
