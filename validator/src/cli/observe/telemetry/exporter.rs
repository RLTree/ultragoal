use serde_json::Value;
use std::process::Command;

const EXPORT_TIMEOUT_SECONDS: &str = "1";

pub(super) fn emit(event: &Value, metric: &Value, trace: &Value) {
    std::thread::scope(|scope| {
        let logs = scope.spawn(|| {
            post_json(
                "http://127.0.0.1:9428/insert/jsonline?_stream_fields=stream,command,law_id,check_id,claim_id,candidate_digest&_msg_field=message&_time_field=timestamp",
                event,
            )
        });
        let metrics = scope.spawn(|| {
            post_text(
                "http://127.0.0.1:8428/api/v1/import/prometheus",
                &super::metric::export::lines(metric),
            )
        });
        let traces = scope.spawn(|| {
            post_json(
                "http://127.0.0.1:10428/insert/opentelemetry/v1/traces",
                &super::trace::export::payload(trace),
            )
        });
        let _ = logs.join();
        let _ = metrics.join();
        let _ = traces.join();
    });
}

fn post_json(url: &str, value: &Value) -> Result<(), String> {
    let body = serde_json::to_string(value).expect("serde_json::Value serialization is infallible");
    post(url, &body, "application/json")
}

fn post_text(url: &str, body: &str) -> Result<(), String> {
    post(url, body, "text/plain")
}

fn post(url: &str, body: &str, content_type: &str) -> Result<(), String> {
    let output = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--max-time",
            EXPORT_TIMEOUT_SECONDS,
            "-H",
            &format!("Content-Type: {content_type}"),
            "-X",
            "POST",
            url,
            "--data-binary",
            body,
        ])
        .output();
    post_output_result(output)
}

fn post_output_result(output: Result<std::process::Output, std::io::Error>) -> Result<(), String> {
    let output = output.map_err(|err| format!("curl launch failed: {err}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "curl export failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

#[cfg(test)]
#[path = "exporter_tests.rs"]
mod tests;

#[cfg(test)]
pub(crate) fn failed_post_message_for_test() -> String {
    post(
        "http://127.0.0.1:9/ultragoal-test",
        "{}",
        "application/json",
    )
    .expect_err("closed local discard port should reject the exporter probe")
}

#[cfg(test)]
pub(crate) fn failed_launch_message_for_test() -> String {
    post_output_result(Err(std::io::Error::other("synthetic curl launch failure")))
        .expect_err("synthetic launch error")
}

#[cfg(test)]
pub(crate) fn metric_line_for_test(metric: &Value) -> String {
    super::metric::export::lines(metric)
}

#[cfg(test)]
pub(crate) fn trace_payload_for_test(trace: &Value) -> Value {
    super::trace::export::payload(trace)
}
