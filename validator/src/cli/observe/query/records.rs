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
