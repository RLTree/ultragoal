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

const RECEIPT_REL: &str = "validation_artifacts/observability/schema-validation.json";

#[derive(Debug)]
pub(crate) struct SchemaValidationCommand {
    pub(crate) schema: Option<String>,
    pub(crate) file: Option<PathBuf>,
    pub(crate) receipt: PathBuf,
    pub(crate) jobs: Option<usize>,
}

pub(crate) fn parse(raw: &[String]) -> Result<Option<SchemaValidationCommand>, String> {
    let args = match raw {
        [first, second, rest @ ..] if first == "schema" && second == "validation" => rest,
        [first, rest @ ..] if first == "schema-validation" => rest,
        _ => return Ok(None),
    };
    let schema = opt_string(args, "--schema");
    let file = opt_path(args, "--file");
    if schema.is_some() != file.is_some() {
        return Err("schema validation requires --schema and --file together".to_string());
    }
    Ok(Some(SchemaValidationCommand {
        schema,
        file,
        receipt: opt_path(args, "--receipt").unwrap_or_else(|| PathBuf::from(RECEIPT_REL)),
        jobs: opt_usize(args, "--jobs")?,
    }))
}

pub(crate) fn run(root: &Path, command: &SchemaValidationCommand) -> Result<i32, String> {
    let started = Instant::now();
    let scheduler = SchedulerConfig::from_jobs(command.jobs)?;
    let store = crate::schema_catalog::load(root);
    let result = if let (Some(schema), Some(file)) = (&command.schema, &command.file) {
        targeted(root, &store, schema, file, scheduler)
    } else {
        mapped(root, &store, scheduler)
    };
    let failures = bootstrap_failures(&store)
        .into_iter()
        .chain(result.failures)
        .collect::<Vec<_>>();
    let status = if failures.is_empty() { "pass" } else { "fail" };
    let receipt_rel = command.receipt.to_string_lossy().to_string();
    let why_failed = claims::why_failed(status, &failures);
    let value = crate::cli::observe::telemetry::command_receipt(
        root,
        crate::cli::observe::telemetry::CommandTelemetry {
            command: "ultragoal schema",
            subcommand: "validation",
            operation: "schema.validation",
            surface: "schema",
            law_id: crate::cli::observe::types::LAW_ID,
            check_id: "schema-validation-observability-binding",
            claim_id: "schema_validation",
            artifact_path: artifact_path(command),
            receipt_path: &receipt_rel,
            status,
            failure_class: if status == "pass" {
                "none"
            } else {
                "schema_validation_failure"
            },
            why_failed: &why_failed,
            where_failed: if status == "pass" {
                "none"
            } else {
                "schema.validation"
            },
            next_repair: claims::next_repair(status),
            claim_impact: claims::impact(status),
            blocked_claims: claims::blocked(),
            supported_claims: claims::supported(status),
            runtime: Some(runtime::from_metrics(
                &result.scheduler_metrics,
                elapsed_ms(started),
                "schema_validation_command_start",
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

fn mapped(
    root: &Path,
    store: &crate::schema_catalog::SchemaStore,
    scheduler: SchedulerConfig,
) -> ValidationResult {
    let results = crate::audit::package::checks::schema_validation_results(root, store, scheduler);
    ValidationResult {
        failures: results.failures,
        scheduler_metrics: results.scheduler_metrics,
    }
}

fn targeted(
    root: &Path,
    store: &crate::schema_catalog::SchemaStore,
    schema: &str,
    file: &Path,
    scheduler: SchedulerConfig,
) -> ValidationResult {
    let root = Arc::new(root.to_path_buf());
    let store = Arc::new(store.clone());
    let schema = schema.to_string();
    let file = file.to_path_buf();
    let task = Box::new(move || validate_one(root.as_ref(), store.as_ref(), &schema, &file));
    let scheduled = crate::scheduler::run_ordered(
        scheduler,
        TaskClass::PureReadParallel,
        vec![task as Box<dyn FnOnce() -> Vec<String> + Send>],
    );
    ValidationResult {
        failures: scheduled.values.into_iter().flatten().collect(),
        scheduler_metrics: vec![scheduled.metrics],
    }
}

fn validate_one(
    root: &Path,
    store: &crate::schema_catalog::SchemaStore,
    schema: &str,
    file: &Path,
) -> Vec<String> {
    let Some(rel) = file.to_str() else {
        return vec![format!(
            "{}: package_path_invalid: non-utf8 path",
            file.display()
        )];
    };
    if let Some(error) = crate::package::inventory::package_path_error(root, rel) {
        return vec![format!("{}: package_path_invalid: {error}", file.display())];
    }
    match crate::json_boundary::read_json(&root.join(file)) {
        Ok(value) => crate::schema_catalog::schema_errors(store, schema, &value)
            .into_iter()
            .map(|err| format!("{}: {err}", file.display()))
            .collect(),
        Err(err) => vec![format!("{}: json_load_failed: {err}", file.display())],
    }
}

fn bootstrap_failures(store: &crate::schema_catalog::SchemaStore) -> Vec<String> {
    store
        .errors
        .iter()
        .map(|err| format!("schema_bootstrap_failed: {err}"))
        .collect()
}

fn write_receipt(root: &Path, receipt: &Path, value: &Value) -> Result<(), String> {
    let path = if receipt.is_absolute() {
        receipt.to_path_buf()
    } else {
        root.join(receipt)
    };
    crate::json_boundary::write_json(&path, value)
}

fn artifact_path(command: &SchemaValidationCommand) -> &str {
    command
        .file
        .as_ref()
        .and_then(|path| path.to_str())
        .unwrap_or("schemas/schema-catalog.json")
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
