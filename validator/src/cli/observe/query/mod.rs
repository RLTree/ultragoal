use crate::cli::observe::telemetry;
use crate::cli::observe::types::ObserveCommand;
use serde_json::Value;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

const NO_MATCHING_ROWS: &str = "observability query returned no matching rows";

mod body;
mod metrics;
mod records;
mod target;
#[cfg(test)]
mod tests;
mod text;
mod transport;
pub(crate) use body::{bounded_rows, candidate_digest_failure, has_matches, observed_failure};
pub(crate) use text::{
    bounded_failure_metric_query_for_operation, bounded_metric_query_for_operation,
    bounded_success_metric_query_for_operation, query_text, trace_tags,
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

pub(crate) fn run(
    root: &Path,
    command: &ObserveCommand,
    query_kind: QueryKind,
) -> Result<Value, String> {
    if command.row_limit == 0 || command.byte_limit == 0 || command.timeout_ms == 0 {
        return telemetry::query_result(
            root,
            command,
            query_kind.label(),
            query_text(command),
            vec![],
            "fail",
            Some("unbounded observability query rejected"),
        );
    }
    let query = metrics::target_query(root, command).unwrap_or_else(|| query_text(command));
    let candidate = crate::package::inventory::package_digest(root)?;
    let output = query_with_retry(root, command, query_kind, &query, &candidate);
    result_from_output_for_candidate(root, command, query_kind, query, output, candidate)
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
    telemetry::query_result(
        root,
        command,
        query_kind.label(),
        query,
        rows,
        status,
        failure.as_deref(),
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
    retry_until_reconciled(command, || live_query(command, query_kind, query), validate)
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

#[cfg(test)]
pub(crate) fn retry_until_match_for_test(
    command: &ObserveCommand,
    mut outputs: Vec<Result<String, String>>,
) -> Result<String, String> {
    outputs.reverse();
    retry_until_match(command, || {
        outputs
            .pop()
            .unwrap_or_else(|| Err("test outputs exhausted".to_string()))
    })
}

#[cfg(test)]
fn retry_until_match<F>(command: &ObserveCommand, fetch: F) -> Result<String, String>
where
    F: FnMut() -> Result<String, String>,
{
    retry_until_reconciled(command, fetch, |body| {
        if has_matches(body, command.operation) {
            None
        } else {
            Some(NO_MATCHING_ROWS.to_string())
        }
    })
}

fn retry_until_reconciled<F, V>(
    command: &ObserveCommand,
    mut fetch: F,
    mut validate: V,
) -> Result<String, String>
where
    F: FnMut() -> Result<String, String>,
    V: FnMut(&str) -> Option<String>,
{
    let deadline = Instant::now() + Duration::from_millis(command.timeout_ms.max(1));
    let mut last_body_failure = None;
    let mut last_error = NO_MATCHING_ROWS.to_string();
    loop {
        match fetch() {
            Ok(body) => match validate(&body) {
                None => return Ok(body),
                Some(failure) => {
                    last_body_failure = Some((body, failure));
                }
            },
            Err(err) => {
                last_error = err;
            }
        }
        if Instant::now() >= deadline {
            return match last_body_failure {
                Some((body, failure)) if failure != NO_MATCHING_ROWS => Ok(body),
                Some((_, failure)) if last_error == NO_MATCHING_ROWS => Err(failure),
                Some(_) | None => Err(last_error),
            };
        }
        thread::sleep(Duration::from_millis(250));
    }
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
    command: &ObserveCommand,
    query_kind: QueryKind,
    query: &str,
) -> Result<String, String> {
    match query_kind {
        QueryKind::Logs => {
            transport::curl("http://127.0.0.1:9428/select/logsql/query", query, command)
        }
        QueryKind::Metrics => transport::curl("http://127.0.0.1:8428/api/v1/query", query, command),
        QueryKind::Traces => transport::curl_traces(command),
    }
}
