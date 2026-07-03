use serde_json::Value;

pub(super) fn audit_failure_summary(
    audit: &Value,
    command_error: Option<&str>,
    code: i32,
) -> String {
    if let Some(err) = command_error {
        return format!("source audit command failed: first_failure={err}");
    }
    let failures = failed_check_details(audit);
    if failures.is_empty() {
        return if code == 0 {
            "none".to_string()
        } else {
            "source audit failed without check details".to_string()
        };
    }
    let (first_check, first_detail) = &failures[0];
    format!(
        "source audit failed checks: total_failures={} first_check={} root_group={} first_detail={}",
        failures.len(),
        first_check,
        root_group(first_detail),
        first_detail.split("; ").next().unwrap_or("no detail")
    )
}

pub(super) fn contract(value: &Value) -> Vec<String> {
    let status = text(value, "status");
    let run_id = text(value, "run_id");
    let operation = text(value, "operation");
    let receipt = text(value, "receipt_path");
    let correlation_id = text(value, "correlation_id");
    let metric_query = crate::cli::observe::query::bounded_metric_query_for_operation(operation);
    let claim_impact = text(value, "claim_impact");
    let mut lines = vec![format!(
        "ultragoal-audit-observe {status} operation={operation} candidate={} receipt={receipt} run_id={run_id} correlation_id={correlation_id} claim_impact={claim_impact} supported_claims={} unsupported_claims={}",
        text(value, "candidate_digest"),
        csv(value.get("supported_claims")),
        csv(value.get("blocked_claims"))
    )];
    if status != "pass" {
        lines.push(format!(
            "failed_law={} failed_check={} why={} where={} claim_impact={claim_impact} next_repair={} receipt={receipt} run_id={run_id} correlation_id={correlation_id} query_logs='ultragoal observe logs query --run-id {run_id} --limit 100' query_metrics='ultragoal observe metrics query --query '{metric_query}' --limit 100' query_traces='ultragoal observe traces query --run-id {run_id} --limit 100'",
            text(value, "law_id"),
            text(value, "check_id"),
            text(value, "why_failed"),
            text(value, "where_failed"),
            text(value, "next_repair")
        ));
    }
    lines
}

fn text<'a>(value: &'a Value, field: &str) -> &'a str {
    value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or("<missing>")
}

fn failed_check_details(audit: &Value) -> Vec<(String, String)> {
    let mut checks = audit
        .get("checks")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|rows| rows.iter())
        .filter_map(|(check, row)| {
            (row.get("status").and_then(Value::as_str) != Some("pass")).then(|| {
                (
                    check.to_string(),
                    row.get("details")
                        .and_then(Value::as_str)
                        .filter(|details| !details.is_empty() && *details != "pass")
                        .unwrap_or("no detail")
                        .to_string(),
                )
            })
        })
        .collect::<Vec<_>>();
    checks.sort();
    checks
}

fn root_group(detail: &str) -> &'static str {
    let stale = detail.contains("candidate_digest_mismatch")
        || detail.contains("target_digest_mismatch")
        || detail.contains("digest_mismatch")
        || detail.contains("source_stale");
    if stale {
        "stale_or_wrong_digest_evidence"
    } else if detail.contains("missing=[") || detail.contains("plugin_inventory") {
        "package_inventory_mismatch"
    } else if detail.contains("observability_") {
        "observability_reconciliation_incomplete"
    } else {
        "source_audit_check_failure"
    }
}

fn csv(value: Option<&Value>) -> String {
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

#[cfg(test)]
#[path = "stdout_tests.rs"]
mod tests;
