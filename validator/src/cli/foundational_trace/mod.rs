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

const RECEIPT_REL: &str = "validation_artifacts/observability/foundational-trace-check.json";
const MATRIX_REL: &str = "docs/source-obligation-matrix.json";
const TRACE_REL: &str = "docs/foundational-law-traceability.json";

#[derive(Debug)]
pub(crate) struct FoundationalTraceCommand {
    pub(crate) obligation: Option<String>,
    pub(crate) receipt: PathBuf,
    pub(crate) jobs: Option<usize>,
}

pub(crate) fn parse(raw: &[String]) -> Result<Option<FoundationalTraceCommand>, String> {
    let args = match raw {
        [first, second, rest @ ..] if first == "foundational-trace" && second == "check" => rest,
        _ => return Ok(None),
    };
    if !args::has_flag(args, "--strict") {
        return Err("foundational-trace check requires --strict".to_string());
    }
    args::reject_unknown(args)?;
    Ok(Some(FoundationalTraceCommand {
        obligation: args::opt_string(args, "--obligation"),
        receipt: args::opt_path(args, "--receipt").unwrap_or_else(|| PathBuf::from(RECEIPT_REL)),
        jobs: args::opt_usize(args, "--jobs")?,
    }))
}

pub(crate) fn run(root: &Path, command: &FoundationalTraceCommand) -> Result<i32, String> {
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
            command: "ultragoal foundational-trace",
            subcommand: "check",
            operation: "foundational-trace.check",
            surface: "foundational_trace",
            law_id: crate::cli::observe::command::LAW_ID,
            check_id: "foundational-trace-check-observability-binding",
            claim_id: "foundational_trace_check",
            artifact_path: TRACE_REL,
            receipt_path: &receipt_rel,
            status,
            failure_class: if status == "pass" {
                "none"
            } else {
                "foundational_trace_check_failure"
            },
            why_failed: &why_failed,
            where_failed: if status == "pass" {
                "none"
            } else {
                "foundational-trace.check"
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
    let trace = match crate::json_boundary::read_json(&root.join(TRACE_REL)) {
        Ok(value) => value,
        Err(err) => {
            return ValidationResult {
                failures: vec![format!("{TRACE_REL}: {err}")],
                scheduler_metrics: Vec::new(),
            };
        }
    };
    let entries = match crate::audit::foundational_law_trace::trace_entries(&trace) {
        Ok(entries) => entries,
        Err(failure) => {
            return ValidationResult {
                failures: vec![failure],
                scheduler_metrics: Vec::new(),
            };
        }
    };
    let rows = rows(&entries, obligation);
    let mut failures = rows.errors;
    let context = crate::audit::foundational_law_trace::trace_context(root, &matrix);
    let scheduled = run_foundational_trace_tasks(root, context, rows.rows, obligation, scheduler);
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

fn rows(entries: &[Value], obligation: Option<&str>) -> Rows {
    if let Some(id) = obligation {
        let selected = entries
            .iter()
            .filter(|row| row.get("obligation_id").and_then(Value::as_str) == Some(id))
            .cloned()
            .collect::<Vec<_>>();
        let errors = if selected.is_empty() {
            vec![format!("foundational_law_trace_missing_obligation:{id}")]
        } else {
            Vec::new()
        };
        Rows {
            rows: selected,
            errors,
        }
    } else {
        Rows {
            rows: entries.to_vec(),
            errors: Vec::new(),
        }
    }
}

fn run_foundational_trace_tasks(
    root: &Path,
    context: crate::audit::foundational_law_trace::TraceContext,
    rows: Vec<Value>,
    obligation: Option<&str>,
    scheduler: SchedulerConfig,
) -> crate::scheduler::Scheduled<Vec<String>> {
    let root = Arc::new(root.to_path_buf());
    let context = Arc::new(context);
    let selected = Arc::new(rows.clone());
    let mut tasks = Vec::new();
    if obligation.is_none() {
        let required_rows = Arc::clone(&selected);
        let required_context = Arc::clone(&context);
        tasks.push(Box::new(move || {
            crate::audit::foundational_law_trace::missing_required_failures(
                required_rows.as_ref(),
                required_context.as_ref(),
            )
        }) as Box<dyn FnOnce() -> Vec<String> + Send>);
    }
    let id_rows = Arc::clone(&selected);
    let id_context = Arc::clone(&context);
    tasks.push(Box::new(move || {
        crate::audit::foundational_law_trace::entry_id_failures(
            id_rows.as_ref(),
            id_context.as_ref(),
        )
    }) as Box<dyn FnOnce() -> Vec<String> + Send>);
    for row in rows {
        let row_root = Arc::clone(&root);
        let row_context = Arc::clone(&context);
        tasks.push(Box::new(move || {
            crate::audit::foundational_law_trace::row_failures(
                row_root.as_ref(),
                &row,
                row_context.as_ref(),
            )
        }) as Box<dyn FnOnce() -> Vec<String> + Send>);
    }
    crate::scheduler::run_ordered(scheduler, TaskClass::PureReadParallel, tasks)
}

fn write_receipt(root: &Path, receipt: &Path, value: &Value) -> Result<(), String> {
    let path =
        crate::output_path::claim_artifact_path(root, receipt, "foundational trace receipt")?;
    crate::json_boundary::write_json(&path, value)
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis())
        .unwrap_or(u64::MAX)
        .max(1)
}
