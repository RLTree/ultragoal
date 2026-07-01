use crate::cli::observe::telemetry;
use crate::cli::observe::types::{ObserveCommand, ObserveOperation};
use serde_json::{Value, json};
use std::path::Path;
use std::process::Command;

const COMPOSE: &str = "dev/observability/compose.yml";
mod health;
#[cfg(test)]
pub(crate) use health::compose_health_row_for_test;
#[cfg(test)]
pub(crate) use health::compose_health_rows_from_result_for_test;
#[cfg(test)]
pub(crate) use health::compose_output_error_for_test;
#[cfg(test)]
pub(crate) use health::health_receipt;

pub(crate) fn run(root: &Path, command: &ObserveCommand) -> Result<Value, String> {
    match command.operation {
        ObserveOperation::StackUp => compose_receipt(root, command, &["up", "-d"]),
        ObserveOperation::StackDown => compose_receipt(root, command, &["down"]),
        ObserveOperation::StackHealth => health::run(root, command),
        ObserveOperation::StackSmoke => smoke(root, command),
        ObserveOperation::StackGcPlan
        | ObserveOperation::StackGcDryRun
        | ObserveOperation::StackGcApply => gc_receipt(root, command),
        _ => telemetry::base_receipt(root, command, "fail", Some("not a stack operation")),
    }
}

fn compose_receipt(root: &Path, command: &ObserveCommand, args: &[&str]) -> Result<Value, String> {
    telemetry::ensure_spool_dir(root)?;
    let output = compose(root, args);
    let status = if output.status { "pass" } else { "fail" };
    let failure = (!output.status).then_some(output.detail.as_str());
    let mut receipt = telemetry::base_receipt(root, command, status, failure)?;
    receipt["stack_command"] = json!({"program":"docker compose","args": args});
    receipt["stack_output"] = json!(output.detail);
    Ok(receipt)
}

fn smoke(root: &Path, command: &ObserveCommand) -> Result<Value, String> {
    let candidate = crate::package::inventory::package_digest(root)?;
    let log = post_log(&candidate, command.timeout_ms);
    let metric = post_metric(&candidate, command.timeout_ms);
    let trace = post_trace_probe(command.timeout_ms);
    smoke_receipt(root, command, &candidate, log, metric, trace)
}

pub(crate) fn smoke_receipt(
    root: &Path,
    command: &ObserveCommand,
    candidate: &str,
    log: bool,
    metric: bool,
    trace: bool,
) -> Result<Value, String> {
    let ok = log && metric && trace;
    let mut receipt = telemetry::base_receipt(
        root,
        command,
        if ok { "pass" } else { "fail" },
        (!ok).then_some("log, metric, and trace smoke probes did not all ingest"),
    )?;
    receipt["smoke"] = json!({
        "victorialogs_ingest": log,
        "victoriametrics_ingest": metric,
        "victoriatraces_probe": trace,
        "candidate_digest": candidate
    });
    Ok(receipt)
}

fn gc_receipt(root: &Path, command: &ObserveCommand) -> Result<Value, String> {
    let mut receipt = telemetry::base_receipt(root, command, "pass", None)?;
    receipt["gc"] = json!({
        "plan_digest": crate::digest::bytes(command.operation.id().as_bytes()),
        "apply_requires_plan_digest": true,
        "deleted_volumes": [],
        "blind_rm_rf_allowed": false
    });
    Ok(receipt)
}

fn compose(root: &Path, args: &[&str]) -> ShellResult {
    let mut command = Command::new("docker");
    command.arg("compose").arg("-f").arg(root.join(COMPOSE));
    command.args(args);
    shell_result(command.output())
}

pub(super) fn curl_ok(url: &str, timeout_ms: u64) -> bool {
    let seconds = timeout_arg(timeout_ms);
    let output = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--max-time",
            &seconds,
            url,
        ])
        .output();
    output.map(|out| out.status.success()).unwrap_or(false)
}

fn post_log(candidate: &str, timeout_ms: u64) -> bool {
    let payload = format!(
        "{{\"stream\":\"ultragoal\",\"message\":\"observability smoke\",\"candidate_digest\":\"{candidate}\",\"timestamp\":\"{}\"}}",
        crate::audit::clock::now_iso()
    );
    let seconds = timeout_arg(timeout_ms);
    Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--max-time",
            &seconds,
            "-X",
            "POST",
            "http://127.0.0.1:9428/insert/jsonline?_stream_fields=stream,candidate_digest&_msg_field=message&_time_field=timestamp",
            "--data-binary",
            &payload,
        ])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn post_metric(candidate: &str, timeout_ms: u64) -> bool {
    let line = format!("ultragoal_stack_smoke_status{{candidate_digest=\"{candidate}\"}} 1\n");
    let seconds = timeout_arg(timeout_ms);
    Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--max-time",
            &seconds,
            "-X",
            "POST",
            "http://127.0.0.1:8428/api/v1/import/prometheus",
            "--data-binary",
            &line,
        ])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn post_trace_probe(timeout_ms: u64) -> bool {
    curl_ok("http://127.0.0.1:10428/health", timeout_ms)
}

fn timeout_arg(timeout_ms: u64) -> String {
    let seconds = (timeout_ms.max(1) as f64 / 1000.0).to_string();
    seconds
}

struct ShellResult {
    status: bool,
    detail: String,
}

fn shell_result(result: std::io::Result<std::process::Output>) -> ShellResult {
    match result {
        Ok(output) => ShellResult {
            status: output.status.success(),
            detail: format!(
                "stdout={} stderr={}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ),
        },
        Err(err) => ShellResult {
            status: false,
            detail: err.to_string(),
        },
    }
}

#[cfg(test)]
pub(crate) fn shell_error_detail_for_test() -> String {
    shell_result(Err(std::io::Error::other("synthetic stack launch failure"))).detail
}
