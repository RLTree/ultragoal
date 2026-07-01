use serde_json::Value;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Instant;

mod claims;
mod runtime;
mod stdout;
#[cfg(test)]
mod tests;
mod validation;

const COVERAGE_RECEIPT_REL: &str = "validation_artifacts/coverage/coverage-receipt.json";
const OBSERVABILITY_RECEIPT_REL: &str = "validation_artifacts/observability/coverage-prove.json";

#[derive(Debug)]
pub(crate) struct CoverageCommand {
    pub(crate) receipt: PathBuf,
    pub(crate) jobs: Option<usize>,
    pub(crate) validate_existing: bool,
}

pub(crate) fn parse(raw: &[String]) -> Result<Option<CoverageCommand>, String> {
    let args = match raw {
        [first, second, rest @ ..] if first == "coverage" && second == "prove" => rest,
        _ => return Ok(None),
    };
    reject_unknown(args)?;
    Ok(Some(CoverageCommand {
        receipt: opt_path(args, "--receipt").unwrap_or_else(|| PathBuf::from(COVERAGE_RECEIPT_REL)),
        jobs: opt_usize(args, "--jobs")?,
        validate_existing: has_flag(args, "--validate-existing"),
    }))
}

pub(crate) fn run(root: &Path, command: &CoverageCommand) -> Result<i32, String> {
    run_with_executor(root, command, execute_authoritative)
}

fn run_with_executor(
    root: &Path,
    command: &CoverageCommand,
    executor: fn(&Path, &Path) -> CoverageExecution,
) -> Result<i32, String> {
    let started = Instant::now();
    let scheduler = crate::scheduler::SchedulerConfig::from_jobs(command.jobs)?;
    let before = crate::package::inventory::package_digest(root)?;
    let execution = if command.validate_existing {
        CoverageExecution::validate_existing()
    } else {
        executor(root, &command.receipt)
    };
    let candidate = crate::package::inventory::package_digest(root)?;
    let mut failures = validation::failures(root, &command.receipt, &candidate);
    if before != candidate {
        failures.push("coverage_command_mutated_package_digest".to_string());
    }
    if execution.code != 0 {
        failures.push(format!(
            "coverage_command_failed:{}",
            first_diagnostic(&execution)
        ));
    }
    failures.sort();
    failures.dedup();
    let status = if failures.is_empty() { "pass" } else { "fail" };
    let why_failed = claims::why_failed(status, &failures);
    let artifact_path = command.receipt.to_string_lossy().to_string();
    let mut value = crate::cli::observe::telemetry::command_receipt(
        root,
        crate::cli::observe::telemetry::CommandTelemetry {
            command: "ultragoal coverage",
            subcommand: "prove",
            operation: "coverage.prove",
            surface: "coverage",
            law_id: crate::cli::observe::types::LAW_ID,
            check_id: "coverage-prove-observability-binding",
            claim_id: "coverage_prove",
            artifact_path: &artifact_path,
            receipt_path: OBSERVABILITY_RECEIPT_REL,
            status,
            failure_class: if status == "pass" {
                "none"
            } else {
                "coverage_prove_failure"
            },
            why_failed: &why_failed,
            where_failed: if status == "pass" {
                "none"
            } else {
                "coverage.prove"
            },
            next_repair: claims::next_repair(status),
            claim_impact: claims::impact(status),
            blocked_claims: claims::blocked(),
            supported_claims: claims::supported(status),
            runtime: Some(runtime::serial(
                elapsed_ms(started),
                execution.cache_mode,
                scheduler.jobs(),
            )),
            emit: true,
        },
    )?;
    value["coverage_receipt_path"] = Value::String(artifact_path);
    write_observability_receipt(root, &value)?;
    stdout::print(&value);
    Ok(i32::from(status != "pass"))
}

struct CoverageExecution {
    code: i32,
    stdout: String,
    stderr: String,
    cache_mode: &'static str,
}

impl CoverageExecution {
    fn validate_existing() -> Self {
        Self {
            code: 0,
            stdout: String::new(),
            stderr: String::new(),
            cache_mode: "coverage_validate_existing_no_cache",
        }
    }
}

fn execute_authoritative(root: &Path, receipt: &Path) -> CoverageExecution {
    let receipt_env = receipt.to_string_lossy().to_string();
    execution_from_output(authoritative_output(root, receipt_env))
}

fn execution_from_output(result: io::Result<Output>) -> CoverageExecution {
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

fn write_observability_receipt(root: &Path, value: &Value) -> Result<(), String> {
    crate::json_boundary::write_json(&root.join(OBSERVABILITY_RECEIPT_REL), value)
}

fn reject_unknown(args: &[String]) -> Result<(), String> {
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--validate-existing" => index += 1,
            "--receipt" | "--jobs" => {
                if index + 1 >= args.len() {
                    return Err(format!("missing value for {}", args[index]));
                }
                index += 2;
            }
            other => return Err(format!("unknown coverage prove argument: {other}")),
        }
    }
    Ok(())
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

fn opt_string(args: &[String], key: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == key)
        .and_then(|index| args.get(index + 1))
        .cloned()
}

fn opt_path(args: &[String], key: &str) -> Option<PathBuf> {
    opt_string(args, key).map(PathBuf::from)
}

fn opt_usize(args: &[String], key: &str) -> Result<Option<usize>, String> {
    let Some(value) = opt_string(args, key) else {
        return Ok(None);
    };
    value
        .parse()
        .map(Some)
        .map_err(|_| format!("invalid numeric value for {key}: {value}"))
}

fn first_diagnostic(execution: &CoverageExecution) -> String {
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

fn is_actionable_diagnostic(line: &str) -> bool {
    line.contains("coverage_")
        || line.contains("error:")
        || line.contains("failed")
        || line.contains("panicked")
}

fn is_noise_diagnostic(line: &str) -> bool {
    line.starts_with("info:") || line.starts_with("warning:")
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis())
        .unwrap_or(u64::MAX)
        .max(1)
}
