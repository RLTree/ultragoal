use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::Instant;

mod claims {
    pub(super) fn impact(status: &str) -> &'static str {
        if status == "pass" {
            "supports_standards_gardener_rebind_source_local_observability_only"
        } else {
            "standards_gardener_rebind_failed_blocks_readiness_release_completion_update_goal"
        }
    }

    pub(super) fn next_repair(status: &str) -> &'static str {
        if status == "pass" {
            "none"
        } else {
            "query this run through observe logs/metrics/traces, repair the standards-gardener receipt path or changed artifact digest, then rerun standards-gardener rebind"
        }
    }

    pub(super) fn supported(status: &str) -> Vec<String> {
        if status == "pass" {
            vec!["standards_gardener_rebind_observability".to_string()]
        } else {
            Vec::new()
        }
    }

    pub(super) fn blocked() -> Vec<String> {
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
}

mod stdout {
    use serde_json::Value;

    pub(super) fn print(value: &Value) {
        for line in contract(value) {
            println!("{line}");
        }
    }

    pub(super) fn contract(value: &Value) -> Vec<String> {
        let status = text(value, "status");
        let run_id = text(value, "run_id");
        let receipt = text(value, "receipt_path");
        let correlation_id = text(value, "correlation_id");
        let claim_impact = text(value, "claim_impact");
        let mut lines = vec![format!(
            "ultragoal-standards-gardener-rebind {status} operation={} candidate={} receipt={receipt} run_id={run_id} correlation_id={correlation_id} claim_impact={claim_impact} supported_claims={} unsupported_claims={}",
            text(value, "operation"),
            text(value, "candidate_digest"),
            csv(value.get("supported_claims")),
            csv(value.get("blocked_claims"))
        )];
        if status != "pass" {
            lines.push(format!(
                "failed_law={} failed_check={} why={} where={} claim_impact={claim_impact} next_repair={} receipt={receipt} run_id={run_id} correlation_id={correlation_id} query_logs='ultragoal observe logs query --run-id {run_id} --limit 100' query_metrics='ultragoal observe metrics query --run-id {run_id} --limit 100' query_traces='ultragoal observe traces query --run-id {run_id} --limit 100'",
                text(value, "law_id"),
                text(value, "check_id"),
                text(value, "why_failed"),
                text(value, "where_failed"),
                text(value, "next_repair")
            ));
        }
        lines
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
}

const OBSERVABILITY_RECEIPT: &str =
    "validation_artifacts/observability/standards-gardener-rebind.json";

#[derive(Debug)]
pub(crate) struct StandardsCommand {
    pub(crate) receipt: PathBuf,
    pub(crate) observability_receipt: PathBuf,
}

pub(crate) fn parse(raw: &[String]) -> Result<Option<StandardsCommand>, String> {
    match raw {
        [a, b, ..] if a == "standards-gardener" && b == "rebind" => Ok(Some(StandardsCommand {
            receipt: opt_path(raw, "--receipt")?,
            observability_receipt: opt_optional_path(raw, "--observability-receipt")?
                .unwrap_or_else(|| PathBuf::from(OBSERVABILITY_RECEIPT)),
        })),
        [a] if a == "standards-gardener" => Ok(None),
        [a, ..] if a == "standards-gardener" => Err("unknown standards-gardener command".into()),
        _ => Ok(None),
    }
}

pub(crate) fn run(root: &Path, command: &StandardsCommand) -> Result<i32, String> {
    let started = Instant::now();
    let rebind = gardener::rebind(root, &command.receipt);
    let status = if rebind.is_ok() { "pass" } else { "fail" };
    let why_failed = rebind.as_ref().err().map(String::as_str).unwrap_or("none");
    let observation =
        observability_receipt(root, command, status, why_failed, elapsed_ms(started))?;
    write_json(root, &command.observability_receipt, &observation)?;
    stdout::print(&observation);
    Ok(i32::from(status != "pass"))
}

fn observability_receipt(
    root: &Path,
    command: &StandardsCommand,
    status: &str,
    why_failed: &str,
    duration_ms: u64,
) -> Result<Value, String> {
    let artifact_path = command.receipt.to_string_lossy().to_string();
    let receipt_path = command.observability_receipt.to_string_lossy().to_string();
    crate::cli::observe::telemetry::command_receipt(
        root,
        crate::cli::observe::telemetry::CommandTelemetry {
            command: "ultragoal standards-gardener",
            subcommand: "rebind",
            operation: "standards-gardener.rebind",
            surface: "standards_gardener",
            law_id: crate::cli::observe::types::LAW_ID,
            check_id: "standards-gardener-rebind-observability-binding",
            claim_id: "standards_gardener_rebind",
            artifact_path: &artifact_path,
            receipt_path: &receipt_path,
            status,
            failure_class: if status == "pass" {
                "none"
            } else {
                "standards_gardener_rebind_failure"
            },
            why_failed,
            where_failed: if status == "pass" {
                "none"
            } else {
                "standards-gardener.rebind"
            },
            next_repair: claims::next_repair(status),
            claim_impact: claims::impact(status),
            blocked_claims: claims::blocked(),
            supported_claims: claims::supported(status),
            runtime: Some(runtime(duration_ms)),
            emit: true,
        },
    )
}

fn runtime(duration_ms: u64) -> crate::cli::observe::telemetry::RuntimeTelemetry {
    crate::cli::observe::telemetry::RuntimeTelemetry {
        duration_ms,
        worker_count: 1,
        task_count: 1,
        queue_depth: 0,
        cpu_ms: None,
        memory_bytes: None,
        io_bytes: None,
        cache_mode: "standards_gardener_authority_write_no_cache".to_string(),
        resource_measurement_status: "wall_time_only_cpu_memory_io_unavailable".to_string(),
        retry_count: 0,
        backoff_ms: 0,
        saturation_status: "shared_authority_write_serial".to_string(),
        repair_anchor_before: "standards_gardener_rebind_start".to_string(),
        repair_anchor_after: "standards_gardener_observability_emit".to_string(),
    }
}

fn write_json(root: &Path, receipt: &Path, value: &Value) -> Result<(), String> {
    let path = if receipt.is_absolute() {
        receipt.to_path_buf()
    } else {
        root.join(receipt)
    };
    crate::json_boundary::write_json(&path, value)
}

fn opt_path(args: &[String], key: &str) -> Result<PathBuf, String> {
    args.iter()
        .position(|arg| arg == key)
        .and_then(|index| args.get(index + 1))
        .map(PathBuf::from)
        .ok_or_else(|| format!("missing required argument {key}"))
}

fn opt_optional_path(args: &[String], key: &str) -> Result<Option<PathBuf>, String> {
    let Some(index) = args.iter().position(|arg| arg == key) else {
        return Ok(None);
    };
    args.get(index + 1)
        .map(PathBuf::from)
        .map(Some)
        .ok_or_else(|| format!("missing required argument {key}"))
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis())
        .unwrap_or(u64::MAX)
        .max(1)
}

pub(crate) mod gardener;
