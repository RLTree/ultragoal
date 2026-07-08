use crate::cli::observe::types::{self, ObserveCommand, ObserveOperation};
use serde_json::Value;
use std::path::Path;

mod query_hint;

pub(super) fn write_and_print(
    root: &Path,
    command: &ObserveCommand,
    value: &Value,
) -> Result<i32, String> {
    let receipt = command.receipt_rel();
    let absolute = crate::output_path::claim_artifact_path(root, &receipt, "observe receipt")?;
    crate::json_boundary::write_json(&absolute, value)?;
    let status = text(value, "status", "fail");
    println!(
        "ultragoal-observe {status} operation={} candidate={} receipt={} run_id={} correlation_id={} trace_id={} failure_class={} claim_impact={} supported_claims={} unsupported_claims={}",
        command.operation.id(),
        text(value, "candidate_digest", "<missing>"),
        receipt.display(),
        text(value, "run_id", "<missing>"),
        text(value, "correlation_id", "<missing>"),
        text(value, "trace_id", "<missing>"),
        text(value, "failure_class", "none"),
        claim_impact(value),
        csv(value.get("supported_claims")),
        csv(value.get("blocked_claims"))
    );
    if is_query(command.operation) {
        print_query(command, value);
    }
    if is_explain(command.operation) {
        print_explain(value);
    }
    if status != "pass" {
        print_failure(command, value, &receipt);
    }
    Ok(i32::from(status != "pass"))
}

fn print_query(command: &ObserveCommand, value: &Value) {
    if command.operation == ObserveOperation::MetricsQuery {
        print_metrics_query(value);
        return;
    }
    println!(
        "query_result kind={} matched={} row_count={} observed_failure_class={} observed_where_failed={} observed_why_failed={} observed_next_repair={} bounded_output={} redaction={} cardinality_guard={} claim_impact={}",
        text(value, "query_kind", "unknown"),
        query_matched(value),
        number_string(value, "row_count"),
        text(value, "observed_failure_class", "none"),
        text(value, "observed_where_failed", "none"),
        text(value, "observed_why_failed", "none"),
        text(value, "observed_next_repair", "none"),
        text(value, "bounded_output_status", "unknown"),
        text(value, "redaction_status", "unknown"),
        text(value, "cardinality_guard", "unknown"),
        claim_impact(value)
    );
}

fn print_metrics_query(value: &Value) {
    println!(
        "query_result kind=metrics matched={} row_count={} operation={} traffic_task_count={} latency_ms={} error_count={} failure_class={} saturation={} labels_bounded={} high_cardinality_labels={} redaction={} claim_impact={}",
        query_matched(value),
        number_string(value, "row_count"),
        text(value, "metric_operation", "unknown"),
        number_string(value, "metric_traffic_task_count"),
        number_string(value, "metric_latency_ms"),
        number_string(value, "metric_error_count"),
        text(value, "metric_failure_class", "none"),
        text(value, "metric_saturation_status", "unknown"),
        text(value, "cardinality_guard", "unknown"),
        text(value, "metric_high_cardinality_labels", "unknown"),
        text(value, "redaction_status", "unknown"),
        claim_impact(value)
    );
}

fn print_failure(command: &ObserveCommand, value: &Value, receipt: &std::path::Path) {
    let metric_query = query_hint::failure_metric_query(command, value);
    println!(
        "failed_check={} failure_class={} why={} where={} claim_impact={} next_repair={} receipt={} run_id={} correlation_id={} trace_id={} query_logs='ultragoal observe logs query --run-id {} --correlation-id {} --limit 100' query_metrics='ultragoal observe metrics query --run-id {} --correlation-id {} --query '{}' --limit 100' query_traces='ultragoal observe traces query --run-id {} --correlation-id {} --limit 100'",
        text(value, "check_id", types::CHECK_ID),
        text(value, "failure_class", "none"),
        text(value, "why_failed", "observability proof failed"),
        text(value, "where_failed", "observe command"),
        claim_impact(value),
        text(value, "next_repair", "run observe stack health and smoke"),
        receipt.display(),
        text(value, "run_id", "unknown"),
        text(value, "correlation_id", "unknown"),
        text(value, "trace_id", "unknown"),
        text(value, "run_id", "unknown"),
        text(value, "correlation_id", "unknown"),
        text(value, "run_id", "unknown"),
        text(value, "correlation_id", "unknown"),
        metric_query,
        text(value, "run_id", "unknown"),
        text(value, "correlation_id", "unknown")
    );
}

fn print_explain(value: &Value) {
    let explanation = value.get("explanation").unwrap_or(&Value::Null);
    let target = value.get("explanation_target").unwrap_or(&Value::Null);
    println!(
        "explain_result requested_target={} fallback_used={} target_status={} root_cause={} where_failed={} why_failed={} implicated_paths={} smallest_repair={} narrow_rerun='{}' broad_rerun='{}' claim_ceiling='{}' query_evidence={}",
        field_string(explanation, "requested_target", "none"),
        field_string(explanation, "fallback_used", "unknown"),
        field_string(target, "status", "unknown"),
        field_string(explanation, "root_cause", "unknown"),
        field_string(target, "where_failed", "unknown"),
        field_string(target, "why_failed", "unknown"),
        array_csv(explanation.get("implicated_paths")),
        field_string(explanation, "smallest_repair", "unknown"),
        field_string(explanation, "narrow_rerun", "unknown"),
        field_string(explanation, "broad_rerun", "unknown"),
        field_string(explanation, "claim_ceiling", "unknown"),
        query_summary(explanation)
    );
}

fn query_summary(explanation: &Value) -> String {
    let evidence = explanation.get("query_evidence").unwrap_or(&Value::Null);
    ["logs", "metrics", "traces"]
        .into_iter()
        .map(|key| format!("{key}:{}", query_status(evidence.get(key))))
        .collect::<Vec<_>>()
        .join(",")
}

fn query_status(value: Option<&Value>) -> String {
    let Some(value) = value else {
        return "missing".to_string();
    };
    let status = value
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("missing");
    if status == "pass" || status == "missing" {
        return status.to_string();
    }
    let why = value
        .get("why_failed")
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty() && *text != "none")
        .unwrap_or("query proof failed");
    format!("{status}({})", clip(why, 120))
}

fn clip(value: &str, limit: usize) -> String {
    if value.len() <= limit {
        value.to_string()
    } else {
        format!("{}...", &value[..limit])
    }
}

fn is_query(operation: ObserveOperation) -> bool {
    matches!(
        operation,
        ObserveOperation::LogsQuery
            | ObserveOperation::MetricsQuery
            | ObserveOperation::TracesQuery
    )
}

fn is_explain(operation: ObserveOperation) -> bool {
    matches!(
        operation,
        ObserveOperation::ExplainNext
            | ObserveOperation::ExplainFailure
            | ObserveOperation::ExplainClaim
            | ObserveOperation::ExplainCheck
            | ObserveOperation::ExplainLaw
    )
}

fn query_matched(value: &Value) -> bool {
    text(value, "status", "fail") == "pass"
        && value
            .get("row_count")
            .and_then(Value::as_u64)
            .is_some_and(|count| count > 0)
}

fn claim_impact(value: &Value) -> &str {
    value
        .get("event")
        .and_then(|event| event.get("claim_impact"))
        .and_then(Value::as_str)
        .or_else(|| value.get("claim_impact").and_then(Value::as_str))
        .unwrap_or("<missing>")
}

fn number_string(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_u64)
        .map(|number| number.to_string())
        .unwrap_or_else(|| "0".to_string())
}

fn field_string(value: &Value, key: &str, default: &str) -> String {
    match value.get(key) {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Bool(flag)) => flag.to_string(),
        Some(Value::Number(number)) => number.to_string(),
        _ => default.to_string(),
    }
}

fn text<'a>(value: &'a Value, field: &str, default: &'a str) -> &'a str {
    value.get(field).and_then(Value::as_str).unwrap_or(default)
}

fn array_csv(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(",")
        })
        .filter(|items| !items.is_empty())
        .unwrap_or_else(|| "none".to_string())
}

fn csv(value: Option<&Value>) -> String {
    array_csv(value)
}

#[cfg(test)]
#[path = "../stdout_tests.rs"]
mod tests;
