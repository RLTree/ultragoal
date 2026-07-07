use crate::cli::observe::query::trace_tags;
use crate::cli::observe::types::ObserveCommand;
use serde::Serialize;
use serde_json::Value;
use std::path::Path;
use std::process::Command;

pub(super) fn curl_traces(
    root: &Path,
    command: &ObserveCommand,
    tags: &str,
) -> Result<String, String> {
    match backend_request(root, command, tags) {
        TraceBackendRequest::TraceById(trace_id) => {
            curl_trace_by_id(command, &trace_id).map_err(|lookup_err| {
                format!("victoriatraces target trace lookup failed: {lookup_err}")
            })
        }
        TraceBackendRequest::Search { tags, operation } => {
            curl_trace_search(command, &tags, operation.as_deref())
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
enum TraceBackendRequest {
    TraceById(String),
    Search {
        tags: String,
        operation: Option<String>,
    },
}

fn backend_request(root: &Path, command: &ObserveCommand, tags: &str) -> TraceBackendRequest {
    target_trace_backend_id(root, command)
        .map(TraceBackendRequest::TraceById)
        .unwrap_or_else(|| TraceBackendRequest::Search {
            tags: tags.to_string(),
            operation: trace_operation(root, command),
        })
}

fn curl_trace_by_id(command: &ObserveCommand, trace_id: &str) -> Result<String, String> {
    let seconds = trace_attempt_timeout_seconds(command.timeout_ms);
    let url = format!("http://127.0.0.1:10428/select/jaeger/api/traces/{trace_id}");
    let output = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--max-time",
            &seconds,
            &url,
        ])
        .output();
    super::transport::curl_result_body(output).map_err(|err| trace_lookup_error(trace_id, err))
}

fn trace_lookup_error(trace_id: &str, err: String) -> String {
    if err.contains("404") {
        format!(
            "victoriatraces trace lookup returned 404 before the span tree was queryable: trace_id={trace_id}"
        )
    } else {
        err
    }
}

fn curl_trace_search(
    command: &ObserveCommand,
    tags: &str,
    operation: Option<&str>,
) -> Result<String, String> {
    let seconds = trace_attempt_timeout_seconds(command.timeout_ms);
    let mut args = vec![
        "--fail".to_string(),
        "--silent".to_string(),
        "--show-error".to_string(),
        "--max-time".to_string(),
        seconds,
        "--get".to_string(),
        "http://127.0.0.1:10428/select/jaeger/api/traces".to_string(),
        "--data-urlencode".to_string(),
        "service=ultragoal".to_string(),
    ];
    if let Some(operation) = operation.filter(|value| !value.is_empty()) {
        args.extend([
            "--data-urlencode".to_string(),
            format!("operation={operation}"),
        ]);
    }
    args.extend([
        "--data-urlencode".to_string(),
        format!("tags={tags}"),
        "--data-urlencode".to_string(),
        "lookback=1h".to_string(),
    ]);
    let output = Command::new("curl").args(args).output();
    super::transport::curl_result_body(output)
}

pub(super) fn trace_query(root: &Path, command: &ObserveCommand) -> String {
    target_trace_tags(root, command).unwrap_or_else(|| trace_tags(command))
}

fn trace_operation(root: &Path, command: &ObserveCommand) -> Option<String> {
    super::target::event(root, command)
        .and_then(|event| tag_value(&event, "operation").map(str::to_string))
}

fn target_trace_backend_id(root: &Path, command: &ObserveCommand) -> Option<String> {
    super::target::event(root, command)
        .and_then(|event| tag_value(&event, "trace_id").map(trace_backend_id))
}

fn target_trace_tags(root: &Path, command: &ObserveCommand) -> Option<String> {
    let event = super::target::event(root, command)?;
    TraceTargetTags::from_event(&event).query_text()
}

#[derive(Serialize)]
struct TraceTargetTags<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    run_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    correlation_id: Option<&'a str>,
}

impl<'a> TraceTargetTags<'a> {
    fn from_event(event: &'a Value) -> Self {
        Self {
            run_id: tag_value(event, "run_id"),
            correlation_id: tag_value(event, "correlation_id"),
        }
    }

    fn query_text(&self) -> Option<String> {
        if self.run_id.is_none() && self.correlation_id.is_none() {
            None
        } else {
            serde_json::to_string(self).ok()
        }
    }
}

fn tag_value<'a>(event: &'a Value, key: &str) -> Option<&'a str> {
    event
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty() && *value != "none")
}

fn trace_backend_id(raw: &str) -> String {
    crate::digest::bytes(raw.as_bytes())
        .trim_start_matches("sha256:")
        .chars()
        .take(32)
        .collect()
}

fn trace_attempt_timeout_seconds(timeout_ms: u64) -> String {
    super::transport::timeout_seconds(timeout_ms.min(1_000))
}

#[cfg(test)]
#[path = "backend_lookup_test_access.rs"]
pub(crate) mod test_access;
