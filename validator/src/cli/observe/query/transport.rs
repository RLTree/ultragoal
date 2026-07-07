use crate::cli::observe::types::ObserveCommand;
use std::process::Command;

pub(super) fn curl(url: &str, query: &str, command: &ObserveCommand) -> Result<String, String> {
    let seconds = timeout_seconds(command.timeout_ms);
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

pub(super) fn curl_traces(
    root: &std::path::Path,
    command: &ObserveCommand,
    tags: &str,
) -> Result<String, String> {
    super::trace_transport::curl_traces(root, command, tags)
}

pub(super) fn curl_metrics(query: &str, command: &ObserveCommand) -> Result<String, String> {
    let output = Command::new("curl")
        .args(metric_query_args(query, command))
        .output();
    curl_result_body(output)
}

fn metric_query_args(query: &str, command: &ObserveCommand) -> Vec<String> {
    vec![
        "--fail".to_string(),
        "--silent".to_string(),
        "--show-error".to_string(),
        "--max-time".to_string(),
        timeout_seconds(command.timeout_ms),
        "--get".to_string(),
        "http://127.0.0.1:8428/api/v1/query".to_string(),
        "--data-urlencode".to_string(),
        format!("query={query}"),
        "--data-urlencode".to_string(),
        format!("time={}", metric_query_time_seconds()),
        "--data-urlencode".to_string(),
        "nocache=1".to_string(),
    ]
}

pub(super) fn timeout_seconds(timeout_ms: u64) -> String {
    (timeout_ms.max(1) as f64 / 1000.0).to_string()
}

fn metric_query_time_seconds() -> i64 {
    crate::audit::clock::parse_iso_seconds(&crate::audit::clock::now_iso()).unwrap_or(0)
}

#[cfg(test)]
pub(crate) fn trace_query_for_test(root: &std::path::Path, command: &ObserveCommand) -> String {
    super::trace_transport::test_access::trace_query_for_test(root, command)
}

#[cfg(test)]
pub(crate) fn trace_operation_for_test(
    root: &std::path::Path,
    command: &ObserveCommand,
) -> Option<String> {
    super::trace_transport::test_access::trace_operation_for_test(root, command)
}

#[cfg(test)]
pub(crate) fn trace_backend_id_for_test(raw: &str) -> String {
    super::trace_transport::test_access::trace_backend_id_for_test(raw)
}

#[cfg(test)]
pub(crate) fn trace_attempt_timeout_seconds_for_test(timeout_ms: u64) -> String {
    super::trace_transport::test_access::trace_attempt_timeout_seconds_for_test(timeout_ms)
}

#[cfg(test)]
pub(crate) fn trace_backend_request_for_test(
    root: &std::path::Path,
    command: &ObserveCommand,
    tags: &str,
) -> String {
    super::trace_transport::test_access::trace_backend_request_for_test(root, command, tags)
}

#[cfg(test)]
pub(crate) fn trace_lookup_error_for_test(trace_id: &str, err: String) -> String {
    super::trace_transport::test_access::trace_lookup_error_for_test(trace_id, err)
}

#[cfg(test)]
pub(crate) fn trace_target_tags_for_test(event: &serde_json::Value) -> Option<String> {
    super::trace_transport::test_access::trace_target_tags_for_test(event)
}

#[cfg(test)]
pub(crate) fn metric_query_args_for_test(query: &str, command: &ObserveCommand) -> Vec<String> {
    metric_query_args(query, command)
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
