use std::io;
use std::path::Path;
use std::process::{Command, Output};

pub(super) struct CoverageExecution {
    pub(super) code: i32,
    pub(super) stdout: String,
    pub(super) stderr: String,
    pub(super) cache_mode: &'static str,
}

impl CoverageExecution {
    pub(super) fn validate_existing() -> Self {
        Self {
            code: 0,
            stdout: String::new(),
            stderr: String::new(),
            cache_mode: "coverage_validate_existing_no_cache",
        }
    }
}

pub(super) fn execute_authoritative(root: &Path, receipt: &Path) -> CoverageExecution {
    let receipt_env = receipt.to_string_lossy().to_string();
    execution_from_output(authoritative_output(root, receipt_env))
}

pub(super) fn execution_from_output(result: io::Result<Output>) -> CoverageExecution {
    match result {
        Ok(output) => CoverageExecution {
            code: output.status.code().unwrap_or(2),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            cache_mode: "coverage_authoritative_no_cache",
        },
        Err(err) => CoverageExecution {
            code: 2,
            stdout: String::new(),
            stderr: format!("coverage_command_spawn_failed:{err}"),
            cache_mode: "coverage_authoritative_no_cache",
        },
    }
}

fn authoritative_output(root: &Path, receipt_env: String) -> io::Result<Output> {
    Command::new("bash")
        .arg("scripts/check-coverage-full")
        .arg(root)
        .current_dir(root)
        .env("HARNESS_COVERAGE_RECEIPT", receipt_env)
        .output()
}

pub(super) fn first_diagnostic(execution: &CoverageExecution) -> String {
    let lines = execution
        .stderr
        .lines()
        .chain(execution.stdout.lines())
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    lines
        .iter()
        .find(|line| is_actionable_diagnostic(line))
        .or_else(|| lines.iter().find(|line| !is_noise_diagnostic(line)))
        .or_else(|| lines.first())
        .copied()
        .unwrap_or("coverage command exited nonzero")
        .to_string()
}

pub(super) fn is_noise_diagnostic(line: &str) -> bool {
    line.starts_with("info:") || line.starts_with("warning:")
}

fn is_actionable_diagnostic(line: &str) -> bool {
    line.contains("coverage_")
        || line.contains("error:")
        || line.contains("failed")
        || line.contains("panicked")
}
