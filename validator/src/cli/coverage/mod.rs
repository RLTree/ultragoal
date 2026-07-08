use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::Instant;

use executor::CoverageExecution;
#[cfg(test)]
use executor::{execution_from_output, first_diagnostic, is_noise_diagnostic};

mod claims;
pub(crate) mod exact_receipt;
mod executor;
mod routine;
mod routine_observation;
mod runtime;
mod stdout;
pub(crate) mod target_dir;
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
    pub(crate) mode: CoverageMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CoverageMode {
    Strict,
    Routine,
}

impl CoverageMode {
    fn parse(value: Option<String>) -> Result<Self, String> {
        match value.as_deref().unwrap_or("strict") {
            "strict" => Ok(Self::Strict),
            "routine" => Ok(Self::Routine),
            other => Err(format!("unknown coverage mode: {other}")),
        }
    }

    fn cache_mode(self, execution: &CoverageExecution) -> &'static str {
        match self {
            Self::Strict => execution.cache_mode,
            Self::Routine => "coverage_routine_verified_local",
        }
    }
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
        mode: CoverageMode::parse(opt_string(args, "--mode"))?,
    }))
}

pub(crate) fn run(root: &Path, command: &CoverageCommand) -> Result<i32, String> {
    run_with_executor(root, command, executor::execute_authoritative)
}

fn run_with_executor(
    root: &Path,
    command: &CoverageCommand,
    executor: fn(&Path, &Path) -> executor::CoverageExecution,
) -> Result<i32, String> {
    let started = Instant::now();
    let scheduler = crate::scheduler::SchedulerConfig::from_jobs(command.jobs)?;
    crate::output_path::claim_artifact_path(root, &command.receipt, "coverage receipt")?;
    let before = crate::package::inventory::package_digest(root)?;
    let execution = match (command.validate_existing, command.mode) {
        (true, _) => CoverageExecution::validate_existing(),
        (false, CoverageMode::Strict) => executor(root, &command.receipt),
        (false, CoverageMode::Routine) => {
            routine_observation::execute(root, &command.receipt, &before)
        }
    };
    let candidate = crate::package::inventory::package_digest(root)?;
    let mut failures = match command.mode {
        CoverageMode::Strict => validation::failures(root, &command.receipt, &candidate),
        CoverageMode::Routine => routine::failures(root, &command.receipt, &candidate),
    };
    if before != candidate {
        failures.push("coverage_command_mutated_package_digest".to_string());
    }
    if execution.code != 0 {
        failures.push(format!(
            "coverage_command_failed:{}",
            executor::first_diagnostic(&execution)
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
            next_repair: claims::next_repair(status, command.mode),
            claim_impact: claims::impact(status, command.mode),
            blocked_claims: claims::blocked(command.mode),
            supported_claims: claims::supported(status, command.mode),
            runtime: Some(runtime::serial(
                elapsed_ms(started),
                command.mode.cache_mode(&execution),
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

fn write_observability_receipt(root: &Path, value: &Value) -> Result<(), String> {
    let receipt = crate::output_path::literal_claim_artifact_path(
        root,
        OBSERVABILITY_RECEIPT_REL,
        "coverage observability receipt",
    );
    crate::json_boundary::write_json(&receipt, value)
}

fn reject_unknown(args: &[String]) -> Result<(), String> {
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--validate-existing" => index += 1,
            "--receipt" | "--jobs" | "--mode" => {
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

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis())
        .unwrap_or(u64::MAX)
        .max(1)
}
