use crate::cli::observe::command::ObserveCommand;
use crate::cli::observe::telemetry;
use serde_json::Value;
use std::path::Path;

mod body;
mod metrics;
mod record_projection;
mod records;
mod retry;
mod target;
#[cfg(test)]
mod tests;
mod text;
mod trace_transport;
mod transport;
pub(crate) use body::{bounded_rows, candidate_digest_failure, has_matches, observed_failure};
pub(crate) use record_projection::observed_telemetry_record;
use retry::NO_MATCHING_ROWS;
#[cfg(test)]
pub(crate) use retry::retry_until_match_for_test;
pub(crate) use retry::retry_until_reconciled;
pub(crate) use text::{
    bounded_failure_metric_query_for_operation, bounded_metric_query_for_operation,
    bounded_metric_query_for_operation_status, bounded_success_metric_query_for_operation,
    query_text, trace_tags,
};
#[cfg(test)]
pub(crate) use transport::{curl_output_body, curl_result_body};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum QueryKind {
    Logs,
    Metrics,
    Traces,
}

impl QueryKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Logs => "logs",
            Self::Metrics => "metrics",
            Self::Traces => "traces",
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct LiveQueryObservation {
    pub(crate) query: String,
    pub(crate) output: Result<String, String>,
    pub(crate) candidate: String,
}

pub(crate) fn run(
    root: &Path,
    command: &ObserveCommand,
    query_kind: QueryKind,
) -> Result<Value, String> {
    let candidate = crate::package::inventory::package_digest(root)?;
    let observation = capture_for_candidate(root, command, query_kind, candidate);
    receipt_from_observation(root, command, query_kind, observation)
}

pub(crate) fn capture_for_candidate(
    root: &Path,
    command: &ObserveCommand,
    query_kind: QueryKind,
    candidate: String,
) -> LiveQueryObservation {
    if command.row_limit == 0 || command.byte_limit == 0 || command.timeout_ms == 0 {
        return LiveQueryObservation {
            query: query_text(command),
            output: Err("unbounded observability query rejected".to_string()),
            candidate,
        };
    }
    let query = target_query(root, command, query_kind).unwrap_or_else(|| query_text(command));
    let output = query_with_retry(root, command, query_kind, &query, &candidate);
    LiveQueryObservation {
        query,
        output,
        candidate,
    }
}

pub(crate) fn receipt_from_observation(
    root: &Path,
    command: &ObserveCommand,
    query_kind: QueryKind,
    observation: LiveQueryObservation,
) -> Result<Value, String> {
    result_from_output_for_candidate(
        root,
        command,
        query_kind,
        observation.query,
        observation.output,
        observation.candidate,
    )
}

#[cfg(test)]
pub(crate) fn result_from_output(
    root: &Path,
    command: &ObserveCommand,
    query_kind: QueryKind,
    query: String,
    output: Result<String, String>,
) -> Result<Value, String> {
    let candidate = crate::package::inventory::package_digest(root)?;
    result_from_output_for_candidate(root, command, query_kind, query, output, candidate)
}

fn result_from_output_for_candidate(
    root: &Path,
    command: &ObserveCommand,
    query_kind: QueryKind,
    query: String,
    output: Result<String, String>,
    candidate: String,
) -> Result<Value, String> {
    let (status, rows, failure) = match output {
        Ok(body) => {
            let rows = bounded_rows(body.clone(), command.byte_limit);
            let failure = match query_kind {
                QueryKind::Metrics => {
                    metrics::reconciliation_failure(root, command, &rows, &candidate)
                }
                QueryKind::Logs | QueryKind::Traces => {
                    records::reconciliation_failure(root, command, query_kind, &rows, &candidate)
                        .or_else(|| candidate_digest_failure(&body, &candidate))
                }
            };
            let status = if failure.is_some() { "fail" } else { "pass" };
            (status, rows, failure)
        }
        Err(err) => ("fail", vec![], Some(err)),
    };
    telemetry::query_result_for_candidate(
        root,
        command,
        query_kind.label(),
        query,
        rows,
        status,
        failure.as_deref(),
        candidate,
    )
}

fn query_with_retry(
    root: &Path,
    command: &ObserveCommand,
    query_kind: QueryKind,
    query: &str,
    candidate: &str,
) -> Result<String, String> {
    let validate = live_result_validator(root, command, query_kind, candidate);
    retry_until_reconciled(
        command,
        || live_query(root, command, query_kind, query),
        validate,
    )
}

fn target_query(root: &Path, command: &ObserveCommand, query_kind: QueryKind) -> Option<String> {
    match query_kind {
        QueryKind::Metrics => metrics::target_query(root, command),
        QueryKind::Traces => Some(trace_transport::trace_query(root, command)),
        QueryKind::Logs => None,
    }
}

fn live_result_validator<'a>(
    root: &'a Path,
    command: &'a ObserveCommand,
    query_kind: QueryKind,
    candidate: &'a str,
) -> impl FnMut(&str) -> Option<String> + 'a {
    move |body| live_result_failure(root, command, query_kind, body, candidate)
}

#[cfg(test)]
pub(crate) fn live_result_validator_for_test<'a>(
    root: &'a Path,
    command: &'a ObserveCommand,
    query_kind: QueryKind,
    candidate: &'a str,
) -> impl FnMut(&str) -> Option<String> + 'a {
    live_result_validator(root, command, query_kind, candidate)
}

fn live_result_failure(
    root: &Path,
    command: &ObserveCommand,
    query_kind: QueryKind,
    body: &str,
    candidate: &str,
) -> Option<String> {
    if !has_matches(body, command.operation) {
        return Some(NO_MATCHING_ROWS.to_string());
    }
    match query_kind {
        QueryKind::Metrics => metrics::reconciliation_failure(
            root,
            command,
            &bounded_rows(body.to_string(), command.byte_limit),
            candidate,
        ),
        QueryKind::Logs | QueryKind::Traces => records::reconciliation_failure(
            root,
            command,
            query_kind,
            &bounded_rows(body.to_string(), command.byte_limit),
            candidate,
        )
        .or_else(|| candidate_digest_failure(body, candidate)),
    }
}

fn live_query(
    root: &Path,
    command: &ObserveCommand,
    query_kind: QueryKind,
    query: &str,
) -> Result<String, String> {
    match query_kind {
        QueryKind::Logs => {
            transport::curl("http://127.0.0.1:9428/select/logsql/query", query, command)
        }
        QueryKind::Metrics => transport::curl_metrics(query, command),
        QueryKind::Traces => transport::curl_traces(root, command, query),
    }
}
