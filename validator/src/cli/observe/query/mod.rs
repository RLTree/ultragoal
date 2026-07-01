use crate::cli::observe::telemetry;
use crate::cli::observe::types::{ObserveCommand, ObserveOperation};
use serde_json::Value;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

mod body;
#[cfg(test)]
mod tests;
mod text;
pub(crate) use body::{bounded_rows, candidate_digest_failure, has_matches, observed_failure};
pub(crate) use text::{bounded_metric_query_for_operation, query_text, trace_tags};

pub(crate) fn run(root: &Path, command: &ObserveCommand) -> Result<Value, String> {
    if command.row_limit == 0 || command.byte_limit == 0 || command.timeout_ms == 0 {
        return telemetry::query_result(
            root,
            command,
            kind(command.operation),
            query_text(command),
            vec![],
            "fail",
            Some("unbounded observability query rejected"),
        );
    }
    let query = query_text(command);
    let output = query_with_retry(command, &query);
    result_from_output(root, command, query, output)
}

pub(crate) fn result_from_output(
    root: &Path,
    command: &ObserveCommand,
    query: String,
    output: Result<String, String>,
) -> Result<Value, String> {
    let candidate = crate::package::inventory::package_digest(root)?;
    let (status, rows, failure) = match output {
        Ok(body) => {
            let failure = match command.operation {
                ObserveOperation::MetricsQuery => None,
                _ => candidate_digest_failure(&body, &candidate),
            };
            let status = if failure.is_some() { "fail" } else { "pass" };
            (status, bounded_rows(body, command.byte_limit), failure)
        }
        Err(err) => ("fail", vec![], Some(err)),
    };
    telemetry::query_result(
        root,
        command,
        kind(command.operation),
        query,
        rows,
        status,
        failure.as_deref(),
    )
}

fn query_with_retry(command: &ObserveCommand, query: &str) -> Result<String, String> {
    retry_until_match(command, || live_query(command, query))
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

fn retry_until_match<F>(command: &ObserveCommand, mut fetch: F) -> Result<String, String>
where
    F: FnMut() -> Result<String, String>,
{
    let deadline = Instant::now() + Duration::from_millis(command.timeout_ms.max(1));
    loop {
        let output = fetch();
        let failure = match output {
            Ok(body) if has_matches(&body, command.operation) => return Ok(body),
            Ok(_) => "observability query returned no matching rows".to_string(),
            Err(err) => err,
        };
        if Instant::now() >= deadline {
            return Err(failure);
        }
        thread::sleep(Duration::from_millis(250));
    }
}

fn live_query(command: &ObserveCommand, query: &str) -> Result<String, String> {
    match command.operation {
        ObserveOperation::LogsQuery => {
            curl("http://127.0.0.1:9428/select/logsql/query", query, command)
        }
        ObserveOperation::MetricsQuery => {
            curl("http://127.0.0.1:8428/api/v1/query", query, command)
        }
        ObserveOperation::TracesQuery => curl_traces(command),
        _ => Err("not an observability query".to_string()),
    }
}

pub(crate) fn kind(operation: ObserveOperation) -> &'static str {
    match operation {
        ObserveOperation::LogsQuery => "logs",
        ObserveOperation::MetricsQuery => "metrics",
        ObserveOperation::TracesQuery => "traces",
        _ => "logs",
    }
}

fn curl(url: &str, query: &str, command: &ObserveCommand) -> Result<String, String> {
    let seconds = (command.timeout_ms.max(1) as f64 / 1000.0).to_string();
    let output = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--max-time",
            &seconds,
            "--get",
            url,
            "--data-urlencode",
            &format!("query={query}"),
        ])
        .output();
    curl_result_body(output)
}

fn curl_traces(command: &ObserveCommand) -> Result<String, String> {
    let seconds = (command.timeout_ms.max(1) as f64 / 1000.0).to_string();
    let tags = trace_tags(command);
    let output = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--max-time",
            &seconds,
            "--get",
            "http://127.0.0.1:10428/select/jaeger/api/traces",
            "--data-urlencode",
            "service=ultragoal",
            "--data-urlencode",
            &format!("tags={tags}"),
        ])
        .output();
    curl_result_body(output)
}

pub(crate) fn curl_result_body(
    output: Result<std::process::Output, std::io::Error>,
) -> Result<String, String> {
    let output = output.map_err(|err| format!("curl launch failed: {err}"))?;
    curl_output_body(output)
}

pub(crate) fn curl_output_body(output: std::process::Output) -> Result<String, String> {
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(format!(
            "curl query failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}
