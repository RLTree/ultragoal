use crate::scheduler::{SchedulerConfig, TaskClass};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::Instant;

mod claims;
mod routine_cache;
mod runtime;
mod stdout;

const RECEIPT_REL: &str = "validation_artifacts/observability/line-cap-check.json";

#[derive(Debug)]
pub(crate) struct LineCapsCommand {
    pub(crate) receipt: PathBuf,
    pub(crate) jobs: Option<usize>,
}

#[cfg(test)]
pub(crate) fn parse(raw: &[String]) -> Result<Option<LineCapsCommand>, String> {
    let args = match raw {
        [first, second, rest @ ..] if first == "line-caps" && second == "check" => rest,
        _ => return Ok(None),
    };
    if !has_flag(args, "--strict") {
        return Err("line-caps check requires --strict".to_string());
    }
    if has_flag(args, "--no-write") && opt_string(args, "--receipt").is_some() {
        return Err("line-caps --no-write conflicts with --receipt".to_string());
    }
    reject_unknown(args)?;
    Ok(Some(LineCapsCommand {
        receipt: if has_flag(args, "--no-write") {
            PathBuf::new()
        } else {
            opt_path(args, "--receipt").unwrap_or_else(|| PathBuf::from(RECEIPT_REL))
        },
        jobs: opt_usize(args, "--jobs")?,
    }))
}

pub(crate) fn run(root: &Path, command: &LineCapsCommand) -> Result<i32, String> {
    let started = Instant::now();
    let scheduler = SchedulerConfig::from_jobs(command.jobs)?;
    let result = validate(root, scheduler);
    let status = if result.failures.is_empty() {
        "pass"
    } else {
        "fail"
    };
    if command.receipt.as_os_str().is_empty() {
        println!(
            "ultragoal-self-law-registry-integrity {status} failures={}",
            result.failures.len()
        );
        for failure in &result.failures {
            println!("ultragoal-self-law-registry-integrity finding={failure}");
        }
        return Ok(i32::from(status != "pass"));
    }
    let receipt_rel = command.receipt.to_string_lossy().to_string();
    let why_failed = claims::why_failed(status, &result.failures);
    let mut value = crate::cli::observe::telemetry::command_receipt(
        root,
        crate::cli::observe::telemetry::CommandTelemetry {
            command: "ultragoal line-caps",
            subcommand: "check --strict",
            operation: "line-caps.check",
            surface: "line_caps",
            law_id: crate::cli::observe::command::LAW_ID,
            check_id: "line-caps-check-observability-binding",
            claim_id: "line_cap_check",
            artifact_path: "validator/src",
            receipt_path: &receipt_rel,
            status,
            failure_class: if status == "pass" {
                "none"
            } else {
                "line_cap_failure"
            },
            why_failed: &why_failed,
            where_failed: if status == "pass" {
                "none"
            } else {
                "line-caps.check"
            },
            next_repair: claims::next_repair(status),
            claim_impact: claims::impact(status),
            blocked_claims: claims::blocked(),
            supported_claims: claims::supported(status),
            runtime: Some(runtime::from_metrics(
                &result.scheduler_metrics,
                elapsed_ms(started),
            )),
            emit: true,
        },
    )?;
    if status == "pass" {
        routine_cache::attach(root, &mut value, command, elapsed_ms(started))?;
    }
    write_receipt(root, &command.receipt, &value)?;
    stdout::print(&value);
    Ok(i32::from(status != "pass"))
}

struct ValidationResult {
    failures: Vec<String>,
    scheduler_metrics: Vec<crate::scheduler::Metrics>,
}

fn validate(root: &Path, scheduler: SchedulerConfig) -> ValidationResult {
    let audit = crate::audit::source_governance::audit(root);
    let inventory = audit.inventory;
    if inventory.sources.is_empty() {
        return ValidationResult {
            failures: vec!["line_cap_no_source_paths".to_string()],
            scheduler_metrics: Vec::new(),
        };
    }
    let scheduled = run_line_cap_tasks(inventory, scheduler);
    let mut failures = audit.failures;
    failures.extend(scheduled.values.into_iter().flatten());
    failures.sort();
    failures.dedup();
    ValidationResult {
        failures,
        scheduler_metrics: vec![scheduled.metrics],
    }
}

pub(crate) fn check(root: &Path, jobs: Option<usize>) -> Result<Vec<String>, String> {
    Ok(validate(root, SchedulerConfig::from_jobs(jobs)?).failures)
}

fn run_line_cap_tasks(
    inventory: crate::audit::source_governance::GovernedInventory,
    scheduler: SchedulerConfig,
) -> crate::scheduler::Scheduled<Vec<String>> {
    let generated_projections = inventory.generated_projections;
    let tasks = inventory
        .sources
        .into_iter()
        .filter(|source| !generated_projections.contains(&source.relative))
        .map(|source| {
            Box::new(move || {
                crate::audit::source_governance::line_cap::failure(&source)
                    .into_iter()
                    .collect::<Vec<_>>()
            }) as Box<dyn FnOnce() -> Vec<String> + Send>
        })
        .collect();
    crate::scheduler::run_ordered(scheduler, TaskClass::PureReadParallel, tasks)
}

fn write_receipt(root: &Path, receipt: &Path, value: &Value) -> Result<(), String> {
    let path = crate::output_path::claim_artifact_path(root, receipt, "line cap receipt")?;
    crate::json_boundary::write_json(&path, value)
}

#[cfg(test)]
fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

#[cfg(test)]
fn reject_unknown(args: &[String]) -> Result<(), String> {
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--strict" | "--no-write" => index += 1,
            "--receipt" | "--jobs" => {
                if index + 1 >= args.len() {
                    return Err(format!("missing value for {}", args[index]));
                }
                index += 2;
            }
            other => return Err(format!("unknown line-caps check argument: {other}")),
        }
    }
    Ok(())
}

#[cfg(test)]
fn opt_string(args: &[String], key: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == key)
        .and_then(|index| args.get(index + 1))
        .cloned()
}

#[cfg(test)]
fn opt_path(args: &[String], key: &str) -> Option<PathBuf> {
    opt_string(args, key).map(PathBuf::from)
}

#[cfg(test)]
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
