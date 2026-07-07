use crate::cli::observe::stack::{COMPOSE, curl_ok};
use crate::cli::observe::telemetry;
use crate::cli::observe::types::ObserveCommand;
use serde_json::{Value, json};
use std::io;
use std::path::Path;
use std::process::{Command, Output};
use std::thread;
use std::time::{Duration, Instant};

const STARTING_HEALTH_POLL_MS: u64 = 500;

pub(crate) fn run(root: &Path, command: &ObserveCommand) -> Result<Value, String> {
    let endpoints = [
        ("victoriametrics", "http://127.0.0.1:8428/health"),
        ("victorialogs", "http://127.0.0.1:9428/health"),
        ("victoriatraces", "http://127.0.0.1:10428/health"),
        ("otel-collector", "http://127.0.0.1:13133/"),
        ("vector", "http://127.0.0.1:8686/health"),
        ("grafana", "http://127.0.0.1:3009/api/health"),
    ];
    let rows = health_rows(root, command, &endpoints);
    health_receipt(root, command, rows)
}

fn health_rows(root: &Path, command: &ObserveCommand, endpoints: &[(&str, &str)]) -> Vec<Value> {
    let mut rows = endpoint_health_rows(command, endpoints);
    rows.extend(compose_health_rows(root));
    wait_for_starting_compose_health(root, command, endpoints, rows)
}

fn endpoint_health_rows(command: &ObserveCommand, endpoints: &[(&str, &str)]) -> Vec<Value> {
    endpoints
        .iter()
        .map(|(name, url)| {
            let ok = curl_ok(url, command.timeout_ms);
            json!({
                "service": name,
                "check_source": "http_endpoint",
                "url": url,
                "status": if ok { "pass" } else { "fail" }
            })
        })
        .collect::<Vec<_>>()
}

pub(super) fn wait_for_starting_compose_health(
    root: &Path,
    command: &ObserveCommand,
    endpoints: &[(&str, &str)],
    mut rows: Vec<Value>,
) -> Vec<Value> {
    let deadline = Instant::now() + Duration::from_millis(command.timeout_ms.max(1));
    while should_retry_starting_health(&rows) && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(STARTING_HEALTH_POLL_MS));
        rows = endpoint_health_rows(command, endpoints);
        rows.extend(compose_health_rows(root));
    }
    rows
}

pub(super) fn should_retry_starting_health(rows: &[Value]) -> bool {
    rows.iter().any(is_starting_compose_row)
        && rows.iter().all(|row| {
            row["status"] == "pass"
                || (row["check_source"] == "docker_compose_ps" && is_starting_compose_row(row))
        })
}

fn is_starting_compose_row(row: &Value) -> bool {
    row["check_source"] == "docker_compose_ps"
        && row["state"] == "running"
        && row["health"] == "starting"
}

pub(crate) fn health_receipt(
    root: &Path,
    command: &ObserveCommand,
    rows: Vec<Value>,
) -> Result<Value, String> {
    let status = if rows.iter().all(|row| row["status"] == "pass") {
        "pass"
    } else {
        "fail"
    };
    let mut receipt = telemetry::base_receipt(
        root,
        command,
        status,
        (status != "pass").then_some("one or more observability stack services failed health"),
    )?;
    receipt["services"] = Value::Array(rows);
    Ok(receipt)
}

fn compose_health_rows(root: &Path) -> Vec<Value> {
    let mut command = Command::new("docker");
    command
        .arg("compose")
        .arg("-f")
        .arg(root.join(COMPOSE))
        .arg("ps")
        .arg("--format")
        .arg("json");
    compose_health_rows_from_result(compose_output(command.output()))
}

fn compose_output(result: io::Result<Output>) -> Result<(bool, String), String> {
    match result {
        Ok(output) => Ok((
            output.status.success(),
            String::from_utf8_lossy(&output.stdout).into_owned(),
        )),
        Err(_) => Err("docker compose ps unavailable".to_string()),
    }
}

fn compose_health_rows_from_result(result: Result<(bool, String), String>) -> Vec<Value> {
    let (success, stdout) = match result {
        Ok(output) => output,
        Err(reason) => return vec![compose_failure(&reason)],
    };
    if !success {
        return vec![compose_failure("docker compose ps returned nonzero")];
    }
    let rows = stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(compose_health_row)
        .collect::<Vec<_>>();
    if rows.is_empty() {
        return vec![compose_failure("docker compose ps returned no services")];
    }
    rows
}

fn compose_health_row(line: &str) -> Value {
    let parsed = serde_json::from_str::<Value>(line);
    let value = match parsed {
        Ok(value) => value,
        Err(_) => return compose_failure("docker compose ps emitted malformed json"),
    };
    let service = text(&value, "Service");
    let name = text(&value, "Name");
    let state = text(&value, "State");
    let health = text(&value, "Health");
    let ok = state == "running" && health == "healthy";
    json!({
        "service": service,
        "container": name,
        "check_source": "docker_compose_ps",
        "state": state,
        "health": health,
        "status": if ok { "pass" } else { "fail" }
    })
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("unknown")
}

fn compose_failure(reason: &str) -> Value {
    json!({
        "service": "docker-compose",
        "check_source": "docker_compose_ps",
        "status": "fail",
        "failure": reason
    })
}

#[cfg(test)]
pub(crate) fn compose_health_row_for_test(line: &str) -> Value {
    compose_health_row(line)
}

#[cfg(test)]
pub(crate) fn compose_health_rows_from_result_for_test(
    result: Result<(bool, String), String>,
) -> Vec<Value> {
    compose_health_rows_from_result(result)
}

#[cfg(test)]
pub(crate) fn compose_output_error_for_test() -> Result<(bool, String), String> {
    compose_output(Err(io::Error::other("missing docker")))
}
