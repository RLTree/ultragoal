use crate::scheduler::{SchedulerConfig, TaskClass};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

mod claims;
mod runtime;
mod stdout;
#[cfg(test)]
mod tests;

const RECEIPT_REL: &str = "validation_artifacts/observability/mandatory-law-validation.json";
const REGISTRY_REL: &str = "docs/mandatory-law-surfaces.json";

#[derive(Debug)]
pub(crate) struct MandatoryLawValidationCommand {
    pub(crate) law: Option<String>,
    pub(crate) receipt: PathBuf,
    pub(crate) jobs: Option<usize>,
}

#[cfg(test)]
pub(crate) fn parse(raw: &[String]) -> Result<Option<MandatoryLawValidationCommand>, String> {
    let args = match raw {
        [first, second, rest @ ..] if first == "mandatory-law" && second == "validation" => rest,
        [first, rest @ ..] if first == "mandatory-law-validation" => rest,
        _ => return Ok(None),
    };
    Ok(Some(MandatoryLawValidationCommand {
        law: opt_string(args, "--law"),
        receipt: opt_path(args, "--receipt").unwrap_or_else(|| PathBuf::from(RECEIPT_REL)),
        jobs: opt_usize(args, "--jobs")?,
    }))
}

pub(crate) fn run(root: &Path, command: &MandatoryLawValidationCommand) -> Result<i32, String> {
    let started = Instant::now();
    let scheduler = SchedulerConfig::from_jobs(command.jobs)?;
    let result = validate(root, command.law.as_deref(), scheduler);
    let status = if result.failures.is_empty() {
        "pass"
    } else {
        "fail"
    };
    let receipt_rel = command.receipt.to_string_lossy().to_string();
    let why_failed = claims::why_failed(status, &result.failures);
    let value = crate::cli::observe::telemetry::command_receipt(
        root,
        crate::cli::observe::telemetry::CommandTelemetry {
            command: "ultragoal mandatory-law",
            subcommand: "validation",
            operation: "mandatory-law.validation",
            surface: "mandatory_law",
            law_id: crate::cli::observe::command::LAW_ID,
            check_id: "mandatory-law-validation-observability-binding",
            claim_id: "mandatory_law_validation",
            artifact_path: REGISTRY_REL,
            receipt_path: &receipt_rel,
            status,
            failure_class: if status == "pass" {
                "none"
            } else {
                "mandatory_law_validation_failure"
            },
            why_failed: &why_failed,
            where_failed: if status == "pass" {
                "none"
            } else {
                "mandatory-law.validation"
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

struct ValidationResult {
    failures: Vec<String>,
    scheduler_metrics: Vec<crate::scheduler::Metrics>,
}

fn validate(root: &Path, law: Option<&str>, scheduler: SchedulerConfig) -> ValidationResult {
    let registry = match crate::json_boundary::read_json(&root.join(REGISTRY_REL)) {
        Ok(value) => value,
        Err(err) => {
            return ValidationResult {
                failures: vec![format!("{REGISTRY_REL}: {err}")],
                scheduler_metrics: Vec::new(),
            };
        }
    };
    let mut failures = registry_failures(root, &registry, law);
    let rows = law_rows(&registry, law, &mut failures);
    let store = crate::schema_catalog::load(root);
    let current = crate::package::inventory::package_digest(root).unwrap_or_default();
    let scheduled = run_law_tasks(root, &store, rows, current, scheduler);
    for failure in scheduled.values.into_iter().flatten() {
        failures.push(failure);
    }
    failures.sort();
    failures.dedup();
    ValidationResult {
        failures,
        scheduler_metrics: vec![scheduled.metrics],
    }
}

fn registry_failures(root: &Path, registry: &Value, law: Option<&str>) -> Vec<String> {
    if law.is_none() {
        crate::audit::mandatory::law::surfaces::value_failures(root, registry)
    } else if registry.get("laws").and_then(Value::as_array).is_none() {
        vec!["mandatory_law_registry_missing_laws".to_string()]
    } else {
        Vec::new()
    }
}

fn law_rows(registry: &Value, law: Option<&str>, failures: &mut Vec<String>) -> Vec<Value> {
    let rows = registry
        .get("laws")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    match law {
        Some(id) => {
            let selected = rows
                .into_iter()
                .filter(|row| row.get("law_id").and_then(Value::as_str) == Some(id))
                .collect::<Vec<_>>();
            if selected.is_empty() {
                failures.push(format!("mandatory_law_missing:{id}"));
            }
            selected
        }
        None => rows,
    }
}

fn run_law_tasks(
    root: &Path,
    store: &crate::schema_catalog::SchemaStore,
    rows: Vec<Value>,
    current_digest: String,
    scheduler: SchedulerConfig,
) -> crate::scheduler::Scheduled<Vec<String>> {
    let root = Arc::new(root.to_path_buf());
    let store = Arc::new(store.clone());
    let tasks = rows
        .into_iter()
        .map(|row| {
            let root = Arc::clone(&root);
            let store = Arc::clone(&store);
            let current_digest = current_digest.clone();
            Box::new(move || validate_law(root.as_ref(), store.as_ref(), row, &current_digest))
                as Box<dyn FnOnce() -> Vec<String> + Send>
        })
        .collect::<Vec<_>>();
    crate::scheduler::run_ordered(scheduler, TaskClass::PureReadParallel, tasks)
}

fn validate_law(
    root: &Path,
    store: &crate::schema_catalog::SchemaStore,
    row: Value,
    current_digest: &str,
) -> Vec<String> {
    let law = row
        .get("law_id")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let mut failures =
        crate::audit::mandatory::law::surfaces::receipt_value_failures_with_candidate(
            root,
            &row,
            current_digest,
        );
    failures.extend(
        crate::audit::mandatory::law::surfaces::anti_theater_dependency_failures(root, store, law),
    );
    failures
}

fn write_receipt(root: &Path, receipt: &Path, value: &Value) -> Result<(), String> {
    let path = crate::output_path::claim_artifact_path(root, receipt, "mandatory law receipt")?;
    crate::json_boundary::write_json(&path, value)
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
