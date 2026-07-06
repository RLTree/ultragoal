use crate::cli::observe::query::QueryKind;
use crate::cli::observe::types::ObserveCommand;
use serde_json::Value;
use std::path::Path;

pub(super) fn reconciliation_failure(
    root: &Path,
    command: &ObserveCommand,
    query_kind: QueryKind,
    rows: &[Value],
    candidate: &str,
) -> Option<String> {
    let Some(event) = super::target::event(root, command) else {
        let requested = super::target::requested(command)?;
        return Some(format!(
            "observability_{}_target_unavailable:{}",
            query_kind.label(),
            requested
        ));
    };
    if let Some(failure) = candidate_failure(query_kind, &event, candidate) {
        return Some(failure);
    }
    if let Some(failure) = target_record_failure(query_kind, &event, rows) {
        return Some(failure);
    }
    target_failure_mismatch(query_kind, &event, super::observed_failure(rows).as_ref())
}

fn candidate_failure(query_kind: QueryKind, event: &Value, candidate: &str) -> Option<String> {
    match text(event, "candidate_digest") {
        Some(value) if value == candidate => None,
        Some(value) => Some(format!(
            "observability_{}_candidate_mismatch:{value}!={candidate}",
            query_kind.label()
        )),
        None => Some(format!(
            "observability_{}_candidate_missing",
            query_kind.label()
        )),
    }
}

fn target_failure_mismatch(
    query_kind: QueryKind,
    event: &Value,
    observed: Option<&Value>,
) -> Option<String> {
    let target_failure = text(event, "failure_class").unwrap_or("none");
    if text(event, "status") == Some("pass") || target_failure == "none" {
        return pass_target_failure(query_kind, observed);
    }
    let Some(observed) = observed else {
        return Some(format!(
            "observability_{}_failure_missing:{target_failure}",
            query_kind.label()
        ));
    };
    let observed_failure = text(observed, "failure_class").unwrap_or("none");
    if observed_failure != target_failure {
        return Some(format!(
            "observability_{}_failure_mismatch:{observed_failure}!={target_failure}",
            query_kind.label()
        ));
    }
    if text(observed, "why_failed").is_none_or(is_empty_or_none) {
        return Some(format!(
            "observability_{}_why_failed_missing:{target_failure}",
            query_kind.label()
        ));
    }
    None
}

fn target_record_failure(query_kind: QueryKind, event: &Value, rows: &[Value]) -> Option<String> {
    let record = super::observed_telemetry_record(rows);
    if record.as_object().is_none_or(|object| object.is_empty()) {
        return Some(format!(
            "observability_{}_target_record_missing:{}",
            query_kind.label(),
            target_label(event)
        ));
    }
    for field in ["run_id", "candidate_digest", "operation"] {
        let expected = text(event, field).unwrap_or("");
        let observed = text(&record, field).unwrap_or("");
        if !expected.is_empty() && observed != expected {
            return Some(format!(
                "observability_{}_target_record_mismatch:{field}:{observed}!={expected}",
                query_kind.label()
            ));
        }
    }
    if let Some(expected) = text(event, "correlation_id").filter(|value| !value.is_empty()) {
        let observed = text(&record, "correlation_id").unwrap_or("");
        if observed != expected {
            return Some(format!(
                "observability_{}_target_record_mismatch:correlation_id:{observed}!={expected}",
                query_kind.label()
            ));
        }
    }
    None
}

fn target_label(event: &Value) -> String {
    text(event, "run_id")
        .map(|run| format!("run_id={run}"))
        .or_else(|| text(event, "check_id").map(|check| format!("check_id={check}")))
        .unwrap_or_else(|| "unknown-target".to_string())
}

fn pass_target_failure(query_kind: QueryKind, observed: Option<&Value>) -> Option<String> {
    let observed_failure = observed
        .and_then(|value| text(value, "failure_class"))
        .unwrap_or("none");
    (observed_failure != "none").then(|| {
        format!(
            "observability_{}_pass_target_has_failure_signal:{observed_failure}",
            query_kind.label()
        )
    })
}

fn text<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(Value::as_str)
}

fn is_empty_or_none(value: &str) -> bool {
    value.is_empty() || value == "none"
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn failure_reconciliation_names_missing_mismatched_and_opaque_observed_failures() {
        let event = json!({
            "status": "fail",
            "failure_class": "coverage_prove_failure"
        });
        assert_eq!(
            target_failure_mismatch(QueryKind::Logs, &event, None),
            Some("observability_logs_failure_missing:coverage_prove_failure".to_string())
        );

        let mismatched = json!({"failure_class": "source_audit_check_failure"});
        assert_eq!(
            target_failure_mismatch(QueryKind::Logs, &event, Some(&mismatched)),
            Some(
                "observability_logs_failure_mismatch:source_audit_check_failure!=coverage_prove_failure"
                    .to_string()
            )
        );

        let opaque = json!({"failure_class": "coverage_prove_failure", "why_failed": "none"});
        assert_eq!(
            target_failure_mismatch(QueryKind::Traces, &event, Some(&opaque)),
            Some("observability_traces_why_failed_missing:coverage_prove_failure".to_string())
        );
    }

    #[test]
    fn target_record_reconciliation_names_identity_mismatches() {
        let event = json!({
            "run_id": "run-target",
            "correlation_id": "corr-target",
            "candidate_digest": "sha256:current",
            "operation": "source.audit"
        });
        assert_eq!(
            target_record_failure(QueryKind::Logs, &event, &[]),
            Some("observability_logs_target_record_missing:run_id=run-target".to_string())
        );

        let wrong_operation = json!({
            "run_id": "run-target",
            "correlation_id": "corr-target",
            "candidate_digest": "sha256:current",
            "operation": "coverage.prove"
        });
        assert_eq!(
            target_record_failure(
                QueryKind::Logs,
                &event,
                &[json!({"body": wrong_operation.to_string()})]
            ),
            Some(
                "observability_logs_target_record_mismatch:operation:coverage.prove!=source.audit"
                    .to_string()
            )
        );

        let wrong_correlation = json!({
            "run_id": "run-target",
            "correlation_id": "corr-other",
            "candidate_digest": "sha256:current",
            "operation": "source.audit"
        });
        assert_eq!(
            target_record_failure(
                QueryKind::Traces,
                &event,
                &[json!({"body": wrong_correlation.to_string()})]
            ),
            Some(
                "observability_traces_target_record_mismatch:correlation_id:corr-other!=corr-target"
                    .to_string()
            )
        );
    }

    #[test]
    fn target_label_uses_check_id_or_unknown_when_run_id_is_absent() {
        assert_eq!(
            target_label(&json!({"check_id":"coverage-current-receipt"})),
            "check_id=coverage-current-receipt"
        );
        assert_eq!(target_label(&json!({})), "unknown-target");
    }
}
