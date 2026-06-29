use crate::cli::observe::telemetry;
use crate::cli::observe::types::{ObserveCommand, ObserveOperation};
use serde_json::{Value, json};
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

mod text;
pub(crate) use text::{query_text, trace_tags};

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
    let (status, rows, failure) = match output {
        Ok(body) => ("pass", bounded_rows(body, command.byte_limit), None),
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

pub(crate) fn bounded_rows(body: String, byte_limit: usize) -> Vec<Value> {
    let redacted = redact_private_paths(&body);
    let clipped = if redacted.len() > byte_limit {
        format!("{}...[truncated]", &redacted[..byte_limit])
    } else {
        redacted
    };
    vec![json!({"body": clipped})]
}

fn redact_private_paths(body: &str) -> String {
    let home_redacted = redact_path_marker(body, private_home_marker(), "[redacted-home-path]");
    redact_path_marker(
        &home_redacted,
        private_tmp_marker(),
        "[redacted-private-tmp-path]",
    )
}

fn private_home_marker() -> &'static str {
    concat!("/", "Users/")
}

fn private_tmp_marker() -> &'static str {
    concat!("/", "private", "/tmp/")
}

fn redact_path_marker(body: &str, marker: &str, replacement: &str) -> String {
    let mut output = String::with_capacity(body.len());
    let mut index = 0;
    while let Some(offset) = body[index..].find(marker) {
        let start = index + offset;
        output.push_str(&body[index..start]);
        let end = body[start..]
            .char_indices()
            .find_map(|(idx, ch)| path_delimiter(ch).then_some(start + idx))
            .unwrap_or(body.len());
        output.push_str(replacement);
        index = end;
    }
    output.push_str(&body[index..]);
    output
}

fn path_delimiter(ch: char) -> bool {
    ch.is_whitespace() || matches!(ch, '"' | '\'' | ',' | '}' | ']' | ')')
}

pub(crate) fn has_matches(body: &str, operation: ObserveOperation) -> bool {
    let body = body.trim();
    if body.is_empty() {
        return false;
    }
    match operation {
        ObserveOperation::MetricsQuery => !body.contains("\"result\":[]"),
        ObserveOperation::TracesQuery => trace_total(body) > 0,
        _ => true,
    }
}

fn trace_total(body: &str) -> i64 {
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| {
            value.get("total").and_then(Value::as_i64).or_else(|| {
                value
                    .get("data")
                    .and_then(Value::as_array)
                    .map(|items| items.len() as i64)
            })
        })
        .unwrap_or(0)
}
