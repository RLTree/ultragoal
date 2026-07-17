use crate::scheduler::{SchedulerConfig, TaskClass};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::Instant;

mod claims;
mod runtime;
mod stdout;

#[cfg(test)]
const RECEIPT_REL: &str = "validation_artifacts/observability/namespace-check.json";

#[derive(Debug)]
pub(crate) struct NamespaceCommand {
    pub(crate) receipt: PathBuf,
    pub(crate) jobs: Option<usize>,
}

#[cfg(test)]
pub(crate) fn parse(raw: &[String]) -> Result<Option<NamespaceCommand>, String> {
    let args = match raw {
        [first, second, rest @ ..] if first == "namespace" && second == "check" => rest,
        _ => return Ok(None),
    };
    if !has_flag(args, "--strict") {
        return Err("namespace check requires --strict".to_string());
    }
    if has_flag(args, "--no-write") && opt_string(args, "--receipt").is_some() {
        return Err("namespace --no-write conflicts with --receipt".to_string());
    }
    reject_unknown(args)?;
    Ok(Some(NamespaceCommand {
        receipt: if has_flag(args, "--no-write") {
            PathBuf::new()
        } else {
            opt_path(args, "--receipt").unwrap_or_else(|| PathBuf::from(RECEIPT_REL))
        },
        jobs: opt_usize(args, "--jobs")?,
    }))
}

pub(crate) fn run(root: &Path, command: &NamespaceCommand) -> Result<i32, String> {
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
            "ultragoal-namespace-no-write {status} failures={}",
            result.failures.len()
        );
        for failure in &result.failures {
            println!("ultragoal-namespace-no-write finding={failure}");
        }
        return Ok(i32::from(status != "pass"));
    }
    let receipt_rel = command.receipt.to_string_lossy().to_string();
    let why_failed = claims::why_failed(status, &result.failures);
    let value = crate::cli::observe::telemetry::command_receipt(
        root,
        crate::cli::observe::telemetry::CommandTelemetry {
            command: "ultragoal namespace",
            subcommand: "check --strict",
            operation: "namespace.check",
            surface: "namespace",
            law_id: "namespace-progressive-disclosure",
            check_id: "namespace-check-observability-binding",
            claim_id: "namespace_check",
            artifact_path: "plugin-manifest-draft.json,docs/namespace-class-registry.json",
            receipt_path: &receipt_rel,
            status,
            failure_class: if status == "pass" {
                "none"
            } else {
                "namespace_law_failure"
            },
            why_failed: &why_failed,
            where_failed: if status == "pass" {
                "none"
            } else {
                "namespace.check"
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
    write_receipt(root, &command.receipt, &value)?;
    stdout::print(&value);
    Ok(i32::from(status != "pass"))
}

#[cfg(test)]
pub(crate) fn stdout_contract_for_test(value: &Value) -> Vec<String> {
    stdout::contract(value)
}

struct ValidationResult {
    failures: Vec<String>,
    scheduler_metrics: Vec<crate::scheduler::Metrics>,
}

fn validate(root: &Path, scheduler: SchedulerConfig) -> ValidationResult {
    let manifest = match crate::json_boundary::read_json(&root.join("plugin-manifest-draft.json")) {
        Ok(value) => value,
        Err(err) => {
            return ValidationResult {
                failures: vec![format!("plugin_manifest_load_failed:{err}")],
                scheduler_metrics: Vec::new(),
            };
        }
    };
    let scheduled = run_namespace_tasks(root, manifest, scheduler);
    let mut failures = scheduled.values.into_iter().flatten().collect::<Vec<_>>();
    failures.sort();
    failures.dedup();
    ValidationResult {
        failures,
        scheduler_metrics: vec![scheduled.metrics],
    }
}

#[cfg(test)]
pub(crate) fn check(root: &Path, jobs: Option<usize>) -> Result<Vec<String>, String> {
    Ok(validate(root, SchedulerConfig::from_jobs(jobs)?).failures)
}

fn run_namespace_tasks(
    root: &Path,
    manifest: Value,
    scheduler: SchedulerConfig,
) -> crate::scheduler::Scheduled<Vec<String>> {
    let root = root.to_path_buf();
    let registry =
        crate::json_boundary::read_json(&root.join("docs/namespace-class-registry.json"));
    let manifest_paths = crate::package::inventory::inventory_paths(&manifest);
    let mut tasks: Vec<Box<dyn FnOnce() -> Vec<String> + Send>> = Vec::new();

    tasks.push({
        let root = root.clone();
        let manifest = manifest.clone();
        Box::new(move || crate::audit::namespace::law::value_failures(&root, &manifest))
    });
    tasks.push({
        let root = root.clone();
        let registry = registry.clone();
        Box::new(move || match registry {
            Ok(value) => crate::audit::namespace::law::class_registry_value_failures(&root, &value),
            Err(err) => vec![format!("namespace_class_registry_file_missing:{err}")],
        })
    });
    tasks.push({
        let registry = registry.clone();
        let manifest_paths = manifest_paths.clone();
        Box::new(move || match registry {
            Ok(value) => {
                crate::audit::namespace::classes::resolution_failures(&value, &manifest_paths)
            }
            Err(_) => Vec::new(),
        })
    });
    tasks.push({
        let root = root.clone();
        Box::new(move || crate::audit::namespace::law::binding_failures(&root))
    });

    crate::scheduler::run_ordered(scheduler, TaskClass::PureReadParallel, tasks)
}

fn write_receipt(root: &Path, receipt: &Path, value: &Value) -> Result<(), String> {
    let path = crate::output_path::claim_artifact_path(root, receipt, "namespace receipt")?;
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
            other => return Err(format!("unknown namespace check argument: {other}")),
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
