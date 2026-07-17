use crate::scheduler::{SchedulerConfig, TaskClass};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::Instant;

#[cfg(test)]
mod boundary_mapping;
mod claim_ceiling;
#[cfg(test)]
mod command_paths;
mod runtime;
mod stdout;

#[cfg(test)]
const RECEIPT_REL: &str = "validation_artifacts/observability/package-inventory.json";

#[derive(Debug)]
pub(crate) struct PackageInventoryCommand {
    pub(crate) receipt: PathBuf,
    pub(crate) jobs: Option<usize>,
}

#[cfg(test)]
pub(crate) fn parse(raw: &[String]) -> Result<Option<PackageInventoryCommand>, String> {
    let args = match raw {
        [first, second, rest @ ..] if first == "package" && second == "inventory" => rest,
        _ => return Ok(None),
    };
    reject_unknown(args)?;
    Ok(Some(PackageInventoryCommand {
        receipt: opt_path(args, "--receipt").unwrap_or_else(|| PathBuf::from(RECEIPT_REL)),
        jobs: opt_usize(args, "--jobs")?,
    }))
}

pub(crate) fn run(root: &Path, command: &PackageInventoryCommand) -> Result<i32, String> {
    let started = Instant::now();
    let scheduler = SchedulerConfig::from_jobs(command.jobs)?;
    let result = validate(root, scheduler);
    let status = if result.failures.is_empty() {
        "pass"
    } else {
        "fail"
    };
    let receipt_rel = command.receipt.to_string_lossy().to_string();
    let why_failed = claim_ceiling::why_failed(status, &result.failures);
    let value = crate::cli::observe::telemetry::command_receipt(
        root,
        crate::cli::observe::telemetry::CommandTelemetry {
            command: "ultragoal package",
            subcommand: "inventory",
            operation: "package.inventory",
            surface: "package_inventory",
            law_id: "cli-control-plane-authority",
            check_id: "plugin-inventory-closure",
            claim_id: "package_inventory_source_local",
            artifact_path: "plugin-manifest-draft.json,docs/plugin-cohesion-manifest.json",
            receipt_path: &receipt_rel,
            status,
            failure_class: if status == "pass" {
                "none"
            } else {
                "package_inventory_failure"
            },
            why_failed: &why_failed,
            where_failed: if status == "pass" {
                "none"
            } else {
                "package.inventory"
            },
            next_repair: claim_ceiling::next_repair(status),
            claim_impact: claim_ceiling::impact(status),
            blocked_claims: claim_ceiling::blocked(),
            supported_claims: claim_ceiling::supported(status),
            runtime: Some(runtime::from_metrics(
                &result.scheduler_metrics,
                elapsed_ms(started),
            )),
            emit: true,
        },
    )?;
    write_receipt(root, &command.receipt, &value)?;
    stdout::print(&value, &result.failures);
    Ok(i32::from(status != "pass"))
}

struct ValidationResult {
    failures: Vec<String>,
    scheduler_metrics: Vec<crate::scheduler::Metrics>,
}

fn validate(root: &Path, scheduler: SchedulerConfig) -> ValidationResult {
    let scheduled = run_inventory_tasks(root, scheduler);
    let mut failures = scheduled.values.into_iter().flatten().collect::<Vec<_>>();
    failures.sort();
    failures.dedup();
    ValidationResult {
        failures,
        scheduler_metrics: vec![scheduled.metrics],
    }
}

fn run_inventory_tasks(
    root: &Path,
    scheduler: SchedulerConfig,
) -> crate::scheduler::Scheduled<Vec<String>> {
    let root = root.to_path_buf();
    let tasks: Vec<Box<dyn FnOnce() -> Vec<String> + Send>> = vec![
        inventory_task(&root, inventory_closure_failures),
        inventory_task(&root, skill_link_failures),
        inventory_task(&root, resource_purpose_failures),
        inventory_task(&root, namespace_inventory_failures),
        Box::new({
            let root = root.clone();
            move || {
                crate::package::inventory::final_bytecode_failures(&root)
                    .into_iter()
                    .map(|failure| format!("plugin-inventory-closure:{failure}"))
                    .collect()
            }
        }),
    ];
    crate::scheduler::run_ordered(scheduler, TaskClass::PureReadParallel, tasks)
}

fn inventory_task(
    root: &Path,
    check: fn(&Path, &Value) -> Vec<String>,
) -> Box<dyn FnOnce() -> Vec<String> + Send> {
    let root = root.to_path_buf();
    Box::new(move || {
        match crate::json_boundary::read_json(&root.join("plugin-manifest-draft.json")) {
            Ok(manifest) => check(&root, &manifest),
            Err(err) => vec![format!(
                "plugin-inventory-closure:plugin-manifest-draft.json load failed: {err}"
            )],
        }
    })
}

fn inventory_closure_failures(root: &Path, manifest: &Value) -> Vec<String> {
    crate::package::inventory::inventory_closure_failures(root, manifest)
        .into_iter()
        .map(|failure| {
            if failure.contains("duplicates=") {
                format!("plugin-inventory-exactly-once:{failure}")
            } else {
                format!("plugin-inventory-closure:{failure}")
            }
        })
        .collect()
}

fn skill_link_failures(root: &Path, manifest: &Value) -> Vec<String> {
    crate::skill_links::manifest_failures(root, manifest)
        .into_iter()
        .map(|failure| {
            format!(
                "skill-inventory-closure:{}:{}",
                failure.code, failure.detail
            )
        })
        .collect()
}

fn resource_purpose_failures(root: &Path, manifest: &Value) -> Vec<String> {
    crate::package::resource::purpose::failures(root, manifest)
        .into_iter()
        .map(|failure| {
            format!(
                "plugin-inventory-closure:{}:{}",
                failure.code, failure.detail
            )
        })
        .collect()
}

fn namespace_inventory_failures(root: &Path, manifest: &Value) -> Vec<String> {
    crate::audit::namespace::law::package_failures(root, manifest)
        .into_iter()
        .map(|failure| format!("namespace-progressive-disclosure:{failure}"))
        .collect()
}

fn write_receipt(root: &Path, receipt: &Path, value: &Value) -> Result<(), String> {
    let path = crate::output_path::claim_artifact_path(root, receipt, "package inventory receipt")?;
    crate::json_boundary::write_json(&path, value)
}

#[cfg(test)]
fn reject_unknown(args: &[String]) -> Result<(), String> {
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--receipt" | "--jobs" => {
                if index + 1 >= args.len() {
                    return Err(format!("missing value for {}", args[index]));
                }
                index += 2;
            }
            other => return Err(format!("unknown package inventory argument: {other}")),
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
