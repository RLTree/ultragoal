use crate::scheduler::{SchedulerConfig, TaskClass};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

mod args;
mod claims;
mod runtime;
mod stdout;
#[cfg(test)]
mod tests;

const RECEIPT_REL: &str = "validation_artifacts/observability/source-obligations-check.json";
const MATRIX_REL: &str = "docs/source-obligation-matrix.json";

#[derive(Debug)]
pub(crate) struct SourceObligationsCommand {
    pub(crate) obligation: Option<String>,
    pub(crate) receipt: PathBuf,
    pub(crate) jobs: Option<usize>,
}

pub(crate) fn parse(raw: &[String]) -> Result<Option<SourceObligationsCommand>, String> {
    let args = match raw {
        [first, second, rest @ ..] if first == "source-obligations" && second == "check" => rest,
        _ => return Ok(None),
    };
    if !args::has_flag(args, "--strict") {
        return Err("source-obligations check requires --strict".to_string());
    }
    args::reject_unknown(args)?;
    Ok(Some(SourceObligationsCommand {
        obligation: args::opt_string(args, "--obligation"),
        receipt: args::opt_path(args, "--receipt").unwrap_or_else(|| PathBuf::from(RECEIPT_REL)),
        jobs: args::opt_usize(args, "--jobs")?,
    }))
}

pub(crate) fn run(root: &Path, command: &SourceObligationsCommand) -> Result<i32, String> {
    let started = Instant::now();
    let scheduler = SchedulerConfig::from_jobs(command.jobs)?;
    let result = validate(root, command.obligation.as_deref(), scheduler);
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
            command: "ultragoal source-obligations",
            subcommand: "check",
            operation: "source-obligations.check",
            surface: "source_obligations",
            law_id: crate::cli::observe::types::LAW_ID,
            check_id: "source-obligations-check-observability-binding",
            claim_id: "source_obligations_check",
            artifact_path: MATRIX_REL,
            receipt_path: &receipt_rel,
            status,
            failure_class: if status == "pass" {
                "none"
            } else {
                "source_obligations_check_failure"
            },
            why_failed: &why_failed,
            where_failed: if status == "pass" {
                "none"
            } else {
                "source-obligations.check"
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

fn validate(root: &Path, obligation: Option<&str>, scheduler: SchedulerConfig) -> ValidationResult {
    let matrix = match crate::json_boundary::read_json(&root.join(MATRIX_REL)) {
        Ok(value) => value,
        Err(err) => {
            return ValidationResult {
                failures: vec![format!("{MATRIX_REL}: {err}")],
                scheduler_metrics: Vec::new(),
            };
        }
    };
    let rows = rows(&matrix, obligation);
    let mut failures = rows.errors;
    let scheduled = run_source_obligation_tasks(root, matrix, rows.rows, obligation, scheduler);
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

struct Rows {
    rows: Vec<Value>,
    errors: Vec<String>,
}

fn rows(matrix: &Value, obligation: Option<&str>) -> Rows {
    let Some(rows) = matrix.get("obligations").and_then(Value::as_array) else {
        return Rows {
            rows: Vec::new(),
            errors: vec!["source_obligation_matrix_missing_rows".to_string()],
        };
    };
    let rows = rows.clone();
    if let Some(id) = obligation {
        let selected = rows
            .into_iter()
            .filter(|row| row.get("id").and_then(Value::as_str) == Some(id))
            .collect::<Vec<_>>();
        let errors = if selected.is_empty() {
            vec![format!("source_obligation_missing:{id}")]
        } else {
            Vec::new()
        };
        Rows {
            rows: selected,
            errors,
        }
    } else {
        Rows {
            rows,
            errors: Vec::new(),
        }
    }
}

fn run_source_obligation_tasks(
    root: &Path,
    matrix: Value,
    rows: Vec<Value>,
    obligation: Option<&str>,
    scheduler: SchedulerConfig,
) -> crate::scheduler::Scheduled<Vec<String>> {
    let root = Arc::new(root.to_path_buf());
    let matrix = Arc::new(matrix);
    let mut tasks = Vec::new();
    if obligation.is_none() {
        let required_rows = rows.clone();
        tasks.push(Box::new(move || {
            crate::audit::source_obligations::required_obligation_failures(&required_rows)
        }) as Box<dyn FnOnce() -> Vec<String> + Send>);
    }
    for row in rows {
        tasks.push(Box::new(move || {
            crate::audit::source_obligations::row_failure(&row)
                .into_iter()
                .collect()
        }) as Box<dyn FnOnce() -> Vec<String> + Send>);
    }
    if obligation.is_none() {
        let trace_root = Arc::clone(&root);
        let trace_matrix = Arc::clone(&matrix);
        tasks.push(Box::new(move || {
            crate::audit::foundational_law_trace::failures(
                trace_root.as_ref(),
                trace_matrix.as_ref(),
            )
        }) as Box<dyn FnOnce() -> Vec<String> + Send>);
        tasks.push(
            Box::new(move || crate::audit::law::family::aliases::failures(root.as_ref()))
                as Box<dyn FnOnce() -> Vec<String> + Send>,
        );
    }
    crate::scheduler::run_ordered(scheduler, TaskClass::PureReadParallel, tasks)
}

fn write_receipt(root: &Path, receipt: &Path, value: &Value) -> Result<(), String> {
    let path =
        crate::output_path::claim_artifact_path(root, receipt, "source obligations receipt")?;
    crate::json_boundary::write_json(&path, value)
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis())
        .unwrap_or(u64::MAX)
        .max(1)
}
