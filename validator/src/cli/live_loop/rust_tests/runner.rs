use serde_json::{Value, json};
use std::path::Path;
use std::process::Command;

#[derive(Debug)]
pub(super) struct RunResult {
    pub(super) status: String,
    pub(super) work_unit_count: usize,
    pub(super) total_executed_test_count: usize,
    pub(super) result_digest: String,
    pub(super) commands: Vec<Value>,
}

pub(super) fn run_filters(root: &Path, filters: &[String]) -> Result<RunResult, String> {
    if filters.is_empty() {
        return Ok(failed_empty_run());
    }
    let mut commands = Vec::new();
    for filter in filters {
        commands.push(run_filter(root, filter)?);
    }
    Ok(result_from_commands(commands))
}

fn run_filter(root: &Path, filter: &str) -> Result<Value, String> {
    if filter.contains('|') {
        return Ok(command_row(
            filter,
            Vec::new(),
            2,
            0,
            "fail",
            "impacted_rust_tests_invalid_or_style_filter",
            b"",
            b"OR-style cargo test filters are unsupported",
        ));
    }
    let argv = vec![
        "cargo".to_string(),
        "test".to_string(),
        "-p".to_string(),
        "ultragoal".to_string(),
        filter.to_string(),
    ];
    let output = Command::new("cargo")
        .current_dir(root)
        .args(["test", "-p", "ultragoal", filter])
        .output()
        .map_err(|err| format!("impacted_rust_tests_runner_launch_failed: {err}"))?;
    Ok(command_row_from_output(
        filter,
        argv,
        output.status.success(),
        output.status.code().unwrap_or(1),
        &output.stdout,
        &output.stderr,
    ))
}

fn result_from_commands(commands: Vec<Value>) -> RunResult {
    let total_executed_test_count = commands
        .iter()
        .filter_map(|row| row.get("executed_test_count").and_then(Value::as_u64))
        .map(|count| count as usize)
        .sum::<usize>();
    let status = if commands
        .iter()
        .all(|row| row.get("status").and_then(Value::as_str) == Some("pass"))
    {
        "pass"
    } else {
        "fail"
    }
    .to_string();
    let result_digest = crate::digest::bytes(
        serde_json::to_string(&commands)
            .unwrap_or_else(|_| "[]".to_string())
            .as_bytes(),
    );
    RunResult {
        status,
        work_unit_count: commands.len(),
        total_executed_test_count,
        result_digest,
        commands,
    }
}

fn failed_empty_run() -> RunResult {
    result_from_commands(vec![command_row(
        "",
        Vec::new(),
        2,
        0,
        "fail",
        "impacted_rust_tests_filter_missing",
        b"",
        b"missing filter",
    )])
}

fn command_row(
    filter: &str,
    argv: Vec<String>,
    exit_code: i32,
    executed_test_count: usize,
    status: &str,
    failure_class: &str,
    stdout: &[u8],
    stderr: &[u8],
) -> Value {
    json!({
        "filter": filter,
        "argv": argv,
        "exit_code": exit_code,
        "executed_test_count": executed_test_count,
        "status": status,
        "failure_class": failure_class,
        "stdout_digest": crate::digest::bytes(stdout),
        "stderr_digest": crate::digest::bytes(stderr),
        "result_digest": crate::digest::bytes(format!(
            "filter={filter};exit={exit_code};executed={executed_test_count};stdout={};stderr={}",
            crate::digest::bytes(stdout),
            crate::digest::bytes(stderr)
        ).as_bytes())
    })
}

fn command_row_from_output(
    filter: &str,
    argv: Vec<String>,
    success: bool,
    exit_code: i32,
    stdout: &[u8],
    stderr: &[u8],
) -> Value {
    let executed = executed_test_count(stdout);
    let failure = if !success {
        "impacted_rust_tests_cargo_failed"
    } else if executed == 0 {
        "impacted_rust_tests_zero_tests_executed"
    } else {
        ""
    };
    command_row(
        filter,
        argv,
        exit_code,
        executed,
        if failure.is_empty() { "pass" } else { "fail" },
        failure,
        stdout,
        stderr,
    )
}

pub(super) fn executed_test_count(stdout: &[u8]) -> usize {
    String::from_utf8_lossy(stdout)
        .lines()
        .filter_map(running_test_count)
        .sum()
}

fn running_test_count(line: &str) -> Option<usize> {
    let rest = line.trim().strip_prefix("running ")?;
    let raw = rest
        .strip_suffix(" tests")
        .or_else(|| rest.strip_suffix(" test"))?;
    raw.parse().ok()
}

#[cfg(test)]
pub(super) fn result_for_test(status: &str, executed_test_count: usize) -> RunResult {
    result_from_commands(vec![command_row(
        "cli::live_loop::rust_tests::",
        vec![
            "cargo".to_string(),
            "test".to_string(),
            "-p".to_string(),
            "ultragoal".to_string(),
            "cli::live_loop::rust_tests::".to_string(),
        ],
        if status == "pass" { 0 } else { 1 },
        executed_test_count,
        status,
        if executed_test_count == 0 {
            "impacted_rust_tests_zero_tests_executed"
        } else {
            ""
        },
        b"running 2 tests\n",
        b"",
    )])
}

#[cfg(test)]
pub(super) fn row_from_output_for_test(success: bool, stdout: &[u8]) -> Value {
    command_row_from_output(
        "cli::live_loop::rust_tests::",
        vec!["cargo".to_string(), "test".to_string()],
        success,
        if success { 0 } else { 1 },
        stdout,
        b"",
    )
}
