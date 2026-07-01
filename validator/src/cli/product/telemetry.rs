use crate::cli::product::{ProductCommand, ProductOperation};
use serde_json::Value;
use std::path::Path;
use std::time::Instant;

pub(super) enum ProductOutcome {
    Report(Value),
    Failure(String),
}

impl ProductOutcome {
    pub(super) fn status(&self) -> &'static str {
        match self {
            Self::Report(report)
                if report.get("status").and_then(Value::as_str) == Some("pass") =>
            {
                "pass"
            }
            Self::Report(_) | Self::Failure(_) => "fail",
        }
    }

    fn failure_class(&self) -> &'static str {
        match self {
            Self::Report(report)
                if report.get("status").and_then(Value::as_str) == Some("pass") =>
            {
                "none"
            }
            Self::Report(_) => "product_receipt_failure",
            Self::Failure(_) => "product_command_failure",
        }
    }

    fn why_failed(&self) -> String {
        match self {
            Self::Report(report)
                if report.get("status").and_then(Value::as_str) == Some("pass") =>
            {
                "none".to_string()
            }
            Self::Report(report) => report
                .get("failures")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join("; ")
                })
                .filter(|items| !items.is_empty())
                .unwrap_or_else(|| "product receipt report failed without details".to_string()),
            Self::Failure(error) => error.clone(),
        }
    }

    fn next_repair(&self) -> &'static str {
        if self.status() == "pass" {
            "none"
        } else {
            "query this run through observe logs/metrics/traces, repair the named product receipt or receipt-dir failure, then rerun product prove-fitness"
        }
    }

    fn claim_impact(&self) -> &'static str {
        if self.status() == "pass" {
            "supports_product_fitness_source_local_observability_only"
        } else {
            "product_fitness_failed_blocks_readiness_release_completion_update_goal"
        }
    }

    fn supported_claims(&self, operation: ProductOperation) -> Vec<String> {
        if self.status() == "pass" {
            vec![operation.claim_id().to_string()]
        } else {
            Vec::new()
        }
    }

    pub(super) fn report(&self) -> Option<&Value> {
        match self {
            Self::Report(report) => Some(report),
            Self::Failure(_) => None,
        }
    }
}

pub(super) fn emit(
    root: &Path,
    command: &ProductCommand,
    started: Instant,
    outcome: &ProductOutcome,
) -> Result<Value, String> {
    let status = outcome.status();
    let why_failed = outcome.why_failed();
    let receipt_path = command.observability_receipt.to_string_lossy().to_string();
    let value = crate::cli::observe::telemetry::command_receipt(
        root,
        crate::cli::observe::telemetry::CommandTelemetry {
            command: command.operation.command(),
            subcommand: command.operation.subcommand(),
            operation: command.operation.telemetry_operation(),
            surface: "product",
            law_id: "product-fitness-gate",
            check_id: command.operation.check_id(),
            claim_id: command.operation.claim_id(),
            artifact_path: "validation_artifacts/harness",
            receipt_path: &receipt_path,
            status,
            failure_class: outcome.failure_class(),
            why_failed: &why_failed,
            where_failed: if status == "pass" {
                "none"
            } else {
                command.operation.telemetry_operation()
            },
            next_repair: outcome.next_repair(),
            claim_impact: outcome.claim_impact(),
            blocked_claims: blocked_claims(),
            supported_claims: outcome.supported_claims(command.operation),
            runtime: Some(product_runtime(started, outcome)),
            emit: true,
        },
    )?;
    crate::json_boundary::write_json(&root.join(&command.observability_receipt), &value)?;
    Ok(value)
}

fn product_runtime(
    started: Instant,
    outcome: &ProductOutcome,
) -> crate::cli::observe::telemetry::RuntimeTelemetry {
    let task_count = outcome
        .report()
        .and_then(|report| report.get("receipts").and_then(Value::as_array))
        .map(|items| items.len())
        .unwrap_or(1);
    crate::cli::observe::telemetry::RuntimeTelemetry {
        duration_ms: u64::try_from(started.elapsed().as_millis())
            .unwrap_or(u64::MAX)
            .max(1),
        worker_count: 1,
        task_count,
        queue_depth: task_count,
        cpu_ms: None,
        memory_bytes: None,
        io_bytes: None,
        cache_mode: "product_receipt_templates_rebound".to_string(),
        resource_measurement_status: "wall_time_only_cpu_memory_io_unavailable".to_string(),
        retry_count: 0,
        backoff_ms: 0,
        saturation_status: "shared_authority_write_serial_product_receipts".to_string(),
        repair_anchor_before: "product_receipt_mint_start".to_string(),
        repair_anchor_after: "product_observability_emit".to_string(),
    }
}

pub(super) fn print_summary(value: &Value) {
    println!(
        "ultragoal-product-observability {} operation={} candidate={} receipt={} run_id={} correlation_id={} claim_impact={} supported_claims={} unsupported_claims={}",
        text(value, "status"),
        text(value, "operation"),
        text(value, "candidate_digest"),
        text(value, "receipt_path"),
        text(value, "run_id"),
        text(value, "correlation_id"),
        text(value, "claim_impact"),
        csv(value.get("supported_claims")),
        csv(value.get("blocked_claims"))
    );
    if text(value, "status") != "pass" {
        println!(
            "failed_law={} failed_check={} why={} where={} next_repair={} run_id={} correlation_id={}",
            text(value, "law_id"),
            text(value, "check_id"),
            text(value, "why_failed"),
            text(value, "where_failed"),
            text(value, "next_repair"),
            text(value, "run_id"),
            text(value, "correlation_id")
        );
    }
}

fn blocked_claims() -> Vec<String> {
    [
        "completion",
        "readiness",
        "release",
        "reviewer_exposure",
        "app_registry_exposure",
        "final_packet_correctness",
        "update_goal_eligibility",
    ]
    .into_iter()
    .map(ToString::to_string)
    .collect()
}

fn text<'a>(value: &'a Value, field: &str) -> &'a str {
    value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or("<missing>")
}

fn csv(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(",")
        })
        .filter(|items| !items.is_empty())
        .unwrap_or_else(|| "none".to_string())
}
