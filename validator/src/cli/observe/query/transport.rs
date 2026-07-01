use crate::cli::observe::query::trace_tags;
use crate::cli::observe::types::ObserveCommand;
use std::process::Command;

pub(super) fn curl(url: &str, query: &str, command: &ObserveCommand) -> Result<String, String> {
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

pub(super) fn curl_traces(command: &ObserveCommand) -> Result<String, String> {
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
