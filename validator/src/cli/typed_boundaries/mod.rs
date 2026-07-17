use crate::scheduler::{SchedulerConfig, TaskClass};
use std::path::{Path, PathBuf};
use std::time::Instant;

mod artifact;
#[cfg(test)]
mod authority_contracts;
mod claims;
#[cfg(test)]
mod command_paths;
mod runtime;
mod stdout;

#[cfg(test)]
const RECEIPT_REL: &str = "validation_artifacts/observability/typed-boundaries-check.json";
const FOUNDATIONAL_SURFACE_INVENTORY_REL: &str =
    "validation_artifacts/package/foundational-surface-inventory.json";
const PACKAGE_SURFACE_INVENTORY_REL: &str =
    "validation_artifacts/package/package-surface-inventory.json";

#[derive(Debug)]
pub(crate) struct TypedBoundariesCommand {
    pub(crate) receipt: PathBuf,
    pub(crate) jobs: Option<usize>,
}

#[cfg(test)]
pub(crate) fn parse(raw: &[String]) -> Result<Option<TypedBoundariesCommand>, String> {
    let args = match raw {
        [first, second, rest @ ..] if first == "typed-boundaries" && second == "check" => rest,
        _ => return Ok(None),
    };
    if !has_flag(args, "--strict") {
        return Err("typed-boundaries check requires --strict".to_string());
    }
    if has_flag(args, "--no-write") && opt_string(args, "--receipt").is_some() {
        return Err("typed-boundaries --no-write conflicts with --receipt".to_string());
    }
    reject_unknown(args)?;
    Ok(Some(TypedBoundariesCommand {
        receipt: if has_flag(args, "--no-write") {
            PathBuf::new()
        } else {
            opt_path(args, "--receipt").unwrap_or_else(|| PathBuf::from(RECEIPT_REL))
        },
        jobs: opt_usize(args, "--jobs")?,
    }))
}

pub(crate) fn run(root: &Path, command: &TypedBoundariesCommand) -> Result<i32, String> {
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
            "ultragoal-typed-boundaries-no-write {status} failures={}",
            result.failures.len()
        );
        for failure in &result.failures {
            println!("ultragoal-typed-boundaries-no-write finding={failure}");
        }
        return Ok(i32::from(status != "pass"));
    }
    let receipt_path = artifact::publication::ClaimReceiptPath::new(root, &command.receipt)?;
    artifact::paths::reject_receipt_collision(&command.receipt)?;
    let receipt_rel = command.receipt.to_string_lossy().to_string();
    let why_failed = claims::why_failed(status, &result.failures);
    let foundational_surface_artifact =
        crate::audit::law::authority_surfaces::foundational_surface_inventory_artifact(root);
    let foundational_write = artifact::publication::write_inventory_artifact(
        root,
        FOUNDATIONAL_SURFACE_INVENTORY_REL,
        &foundational_surface_artifact,
        "foundational surface inventory artifact",
    )?;
    let foundational_surface_binding =
        crate::audit::law::authority_surfaces::FoundationalSurfaceArtifactBinding {
            path: FOUNDATIONAL_SURFACE_INVENTORY_REL.to_string(),
            digest: foundational_write.digest,
            status: foundational_write.status,
        };
    let package_surface_artifact =
        crate::audit::law::authority_surfaces::package_surface_inventory_artifact(root);
    let artifact_write = artifact::publication::write_inventory_artifact(
        root,
        PACKAGE_SURFACE_INVENTORY_REL,
        &package_surface_artifact,
        "package surface inventory artifact",
    )?;
    let package_surface_binding =
        crate::audit::law::authority_surfaces::PackageSurfaceArtifactBinding {
            path: PACKAGE_SURFACE_INVENTORY_REL.to_string(),
            digest: artifact_write.digest,
            status: artifact_write.status,
        };
    let mut value = crate::cli::observe::telemetry::command_receipt(
        root,
        crate::cli::observe::telemetry::CommandTelemetry {
            command: "ultragoal typed-boundaries",
            subcommand: "check --strict",
            operation: "typed-boundaries.check",
            surface: "typed_boundaries",
            law_id: "authority-source-binding",
            check_id: "typed-boundaries-check-observability-binding",
            claim_id: "typed_boundary_check",
            artifact_path: "docs/mandatory-law-surfaces.json,plugin-manifest-draft.json,validator/src/audit/law/authority_surfaces",
            receipt_path: &receipt_rel,
            status,
            failure_class: if status == "pass" {
                "none"
            } else {
                "typed_authority_boundary_failure"
            },
            why_failed: &why_failed,
            where_failed: if status == "pass" {
                "none"
            } else {
                "typed-boundaries.check"
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
    let surface_inventory =
        crate::audit::law::authority_surfaces::foundational_surface_inventory_with_artifacts(
            root,
            &foundational_surface_binding,
            &package_surface_binding,
        );
    value["foundational_law_surface_inventory"] = surface_inventory.clone();
    value["event"]["foundational_law_surface_inventory"] = surface_inventory;
    artifact::publication::write_receipt(&receipt_path, &value)?;
    stdout::print(&value);
    Ok(i32::from(status != "pass"))
}

struct ValidationResult {
    failures: Vec<String>,
    scheduler_metrics: Vec<crate::scheduler::Metrics>,
}

fn validate(root: &Path, scheduler: SchedulerConfig) -> ValidationResult {
    let scheduled = run_typed_boundary_tasks(root, scheduler);
    let mut failures = scheduled
        .values
        .into_iter()
        .flatten()
        .map(|(check, detail)| format!("{check}:{detail}"))
        .collect::<Vec<_>>();
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

fn run_typed_boundary_tasks(
    root: &Path,
    scheduler: SchedulerConfig,
) -> crate::scheduler::Scheduled<Vec<(String, String)>> {
    let root = root.to_path_buf();
    let authority_root = root.clone();
    let tasks: Vec<Box<dyn FnOnce() -> Vec<(String, String)> + Send>> = vec![
        Box::new(move || crate::audit::law::authority_surfaces::package_failures(&authority_root)),
        Box::new(move || {
            crate::audit::plugin::dependency_adapter::failures(&root)
                .into_iter()
                .map(|failure| {
                    (
                        "third-party-dependency-legibility-typed-adapters".to_string(),
                        failure,
                    )
                })
                .collect()
        }),
    ];
    crate::scheduler::run_ordered(scheduler, TaskClass::PureReadParallel, tasks)
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
            other => return Err(format!("unknown typed-boundaries check argument: {other}")),
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
