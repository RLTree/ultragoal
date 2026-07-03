mod context;
mod graph;
mod receipt;
mod surfaces;
#[cfg(test)]
mod tests;

use context::AuditContext;
use receipt::loop_receipt;

use crate::scheduler::{SchedulerConfig, TaskClass};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Debug)]
pub(crate) struct LiveLoopCommand {
    pub(crate) tier: String,
    pub(crate) cache_mode: String,
    pub(crate) jobs: Option<usize>,
    pub(crate) receipt: PathBuf,
}

pub(crate) fn parse(raw: &[String]) -> Result<Option<LiveLoopCommand>, String> {
    match raw {
        [a, b, ..] if a == "loop" && b == "run" => Ok(Some(LiveLoopCommand {
            tier: opt_string(&raw[2..], "--tier").unwrap_or_else(|| "hot".to_string()),
            cache_mode: opt_string(&raw[2..], "--cache-mode")
                .unwrap_or_else(|| "verified-local".to_string()),
            jobs: opt_jobs(&raw[2..], "--jobs")?,
            receipt: opt_path(&raw[2..], "--receipt").unwrap_or_else(|| {
                PathBuf::from("validation_artifacts/observability/loop-run.json")
            }),
        })),
        _ => Ok(None),
    }
}

pub(crate) fn run(root: &Path, command: &LiveLoopCommand) -> Result<i32, String> {
    let started = Instant::now();
    let candidate = crate::package::inventory::package_digest(root)?;
    let config = SchedulerConfig::from_jobs(command.jobs)?;
    let context = AuditContext::new(root, candidate.clone(), command);
    let scheduled =
        crate::scheduler::run_ordered(config, TaskClass::PureReadParallel, context.tasks());
    let current_state = crate::cli::current_state::snapshot_for_candidate(root, candidate.clone());
    let current_state_path = root.join("validation_artifacts/current-state.json");
    crate::json_boundary::write_json(&current_state_path, &current_state)?;
    let first_blocker = current_state["first_blocker"].clone();
    let status = if first_blocker.get("id").and_then(Value::as_str) == Some("none") {
        "pass"
    } else {
        "fail"
    };
    let receipt_result = loop_receipt(
        root,
        command,
        context,
        scheduled,
        current_state,
        status,
        started,
    );
    let receipt = receipt_result?;
    let path =
        crate::output_path::claim_artifact_path(root, &command.receipt, "live loop receipt")?;
    crate::json_boundary::write_json(&path, &receipt)?;
    print_summary(command, &receipt, &first_blocker);
    Ok(i32::from(status != "pass"))
}

fn print_summary(command: &LiveLoopCommand, receipt: &Value, blocker: &Value) {
    println!(
        "ultragoal-loop {} candidate={} tier={} cache_mode={} duration_ms={} worker_count={} task_count={} queue_depth={} critical_path='{}' first_blocker={} why={} next_repair={} narrow_rerun='{}' broad_rerun='{}' claim_ceiling='{}' receipt={} run_id={} correlation_id={}",
        text(receipt, "status", "fail"),
        text(receipt, "candidate_digest", "<missing>"),
        command.tier,
        command.cache_mode,
        number(receipt, "duration_ms"),
        number(receipt, "worker_count"),
        number(receipt, "task_count"),
        number(receipt, "queue_depth"),
        text(receipt, "critical_path", "unknown"),
        text(blocker, "id", "unknown"),
        text(blocker, "why_failed", "unknown"),
        text(blocker, "next_repair", "unknown"),
        text(blocker, "narrow_rerun", "unknown"),
        text(blocker, "broad_rerun", "unknown"),
        text(receipt, "claim_ceiling", "source-local only"),
        command.receipt.display(),
        text(&receipt["observability"], "run_id", "unknown"),
        text(&receipt["observability"], "correlation_id", "unknown")
    );
}

fn opt_string(args: &[String], key: &str) -> Option<String> {
    args.windows(2)
        .find(|window| window[0] == key)
        .map(|window| window[1].clone())
}

fn opt_path(args: &[String], key: &str) -> Option<PathBuf> {
    opt_string(args, key).map(PathBuf::from)
}

fn opt_jobs(args: &[String], key: &str) -> Result<Option<usize>, String> {
    match opt_string(args, key).as_deref() {
        None | Some("auto") => Ok(None),
        Some(raw) => raw
            .parse()
            .map(Some)
            .map_err(|_| format!("invalid --jobs value: {raw}")),
    }
}

fn text<'a>(value: &'a Value, key: &str, default: &'a str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or(default)
}

fn number(value: &Value, key: &str) -> u64 {
    value.get(key).and_then(Value::as_u64).unwrap_or(0)
}
