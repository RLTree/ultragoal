use super::{event, query};
use serde_json::{Value, json};

#[derive(Clone, Debug)]
pub(super) struct ReconciliationReport {
    pub(super) status: String,
    pub(super) value: Value,
}

pub(super) fn from_roundtrips(
    observation: event::CommandObservation,
    observation_path: std::path::PathBuf,
    logs: query::ObserveReceipt,
    metrics: query::ObserveReceipt,
    traces: query::ObserveReceipt,
    explain: query::ObserveReceipt,
) -> ReconciliationReport {
    let status = reconciliation_status(all_roundtrip_statuses_pass(
        &logs, &metrics, &traces, &explain,
    ));
    let roundtrip_durations = roundtrip_durations(&logs, &metrics, &traces, &explain);
    let slowest_roundtrip = slowest_roundtrip(&logs, &metrics, &traces, &explain);
    let first_failed_roundtrip = first_failed_roundtrip(&logs, &metrics, &traces, &explain);
    ReconciliationReport {
        status: status.to_string(),
        value: json!({
            "status": status,
            "run_id": observation.run_id,
            "correlation_id": observation.correlation_id,
            "trace_id": observation.trace_id,
            "command_observation_receipt": observation_path.display().to_string(),
            "command_observation": observation.value(),
            "logs_query": logs.value(),
            "metrics_query": metrics.value(),
            "traces_query": traces.value(),
            "explain_failure": explain.value(),
            "roundtrip_durations_ms": roundtrip_durations,
            "slowest_roundtrip": slowest_roundtrip,
            "first_failed_roundtrip": first_failed_roundtrip,
            "claim_impact": "source_local_live_loop_node_observation_only_not_speed_claim"
        }),
    }
}

pub(super) fn query_receipt_is_claim_observable(receipt: &query::ObserveReceipt) -> bool {
    receipt.exit_code == 0
        && receipt.status == "pass"
        && text(&receipt.value, "status") == Some("pass")
        && text(&receipt.value, "bounded_output_status") == Some("pass")
        && text(&receipt.value, "redaction_status") == Some("pass")
        && receipt
            .value
            .get("row_count")
            .and_then(Value::as_u64)
            .is_some_and(|rows| rows > 0)
        && text(&receipt.value, "result_digest").is_some_and(nonempty)
}

fn all_roundtrip_statuses_pass(
    logs: &query::ObserveReceipt,
    metrics: &query::ObserveReceipt,
    traces: &query::ObserveReceipt,
    explain: &query::ObserveReceipt,
) -> bool {
    query_receipt_is_claim_observable(logs)
        && query_receipt_is_claim_observable(metrics)
        && query_receipt_is_claim_observable(traces)
        && explain_receipt_is_claim_observable(explain)
}

fn roundtrip_durations(
    logs: &query::ObserveReceipt,
    metrics: &query::ObserveReceipt,
    traces: &query::ObserveReceipt,
    explain: &query::ObserveReceipt,
) -> Value {
    json!({
        "logs_query": logs.duration_ms,
        "metrics_query": metrics.duration_ms,
        "traces_query": traces.duration_ms,
        "explain_failure": explain.duration_ms
    })
}

fn slowest_roundtrip(
    logs: &query::ObserveReceipt,
    metrics: &query::ObserveReceipt,
    traces: &query::ObserveReceipt,
    explain: &query::ObserveReceipt,
) -> Value {
    let mut slowest = ("logs_query", logs.duration_ms);
    for candidate in [
        ("metrics_query", metrics.duration_ms),
        ("traces_query", traces.duration_ms),
        ("explain_failure", explain.duration_ms),
    ] {
        if candidate.1 > slowest.1 {
            slowest = candidate;
        }
    }
    json!({"roundtrip": slowest.0, "duration_ms": slowest.1})
}

fn first_failed_roundtrip(
    logs: &query::ObserveReceipt,
    metrics: &query::ObserveReceipt,
    traces: &query::ObserveReceipt,
    explain: &query::ObserveReceipt,
) -> Value {
    for (name, receipt, observable) in [
        ("logs_query", logs, query_receipt_is_claim_observable(logs)),
        (
            "traces_query",
            traces,
            query_receipt_is_claim_observable(traces),
        ),
        (
            "metrics_query",
            metrics,
            query_receipt_is_claim_observable(metrics),
        ),
        (
            "explain_failure",
            explain,
            explain_receipt_is_claim_observable(explain),
        ),
    ] {
        if !observable {
            return json!({
                "roundtrip": name,
                "status": receipt.status,
                "exit_code": receipt.exit_code,
                "duration_ms": receipt.duration_ms,
                "failure": text(&receipt.value, "failure").unwrap_or("none"),
                "claim_impact": text(&receipt.value, "claim_impact")
                    .unwrap_or("observability_reconciliation_blocked")
            });
        }
    }
    Value::Null
}

fn explain_receipt_is_claim_observable(receipt: &query::ObserveReceipt) -> bool {
    receipt.exit_code == 0
        && receipt.status == "pass"
        && text(&receipt.value, "status") == Some("pass")
        && text(&receipt.value, "bounded_output_proof") == Some("pass")
        && text(&receipt.value, "failure_class").is_some()
        && text(&receipt.value, "claim_impact").is_some_and(nonempty)
        && explain_query_evidence_is_present(&receipt.value)
}

fn explain_query_evidence_is_present(value: &Value) -> bool {
    let Some(evidence) = value
        .get("explanation")
        .and_then(|explanation| explanation.get("query_evidence"))
    else {
        return false;
    };
    ["logs", "metrics", "traces"].iter().all(|key| {
        evidence
            .get(*key)
            .and_then(|query| query.get("status"))
            .and_then(Value::as_str)
            .is_some_and(|status| status != "missing")
    })
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn nonempty(value: &str) -> bool {
    !value.is_empty()
}

fn reconciliation_status(roundtrips_passed: bool) -> &'static str {
    match roundtrips_passed {
        true => "pass",
        false => "query_or_explain_reconciliation_failed",
    }
}
