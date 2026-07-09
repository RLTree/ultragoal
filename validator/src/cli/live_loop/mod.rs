mod blockers;
pub(crate) mod changed_inputs;
#[cfg(test)]
mod command_registry_tests;
mod context;
mod graph;
#[cfg(test)]
mod loop_status_projection_tests;
mod node_timing_refresh;
mod nodes;
#[cfg(test)]
mod parser_tests;
#[cfg(test)]
mod projection_tests;
mod receipt;
#[cfg(test)]
mod receipt_tests;
#[cfg(test)]
mod run_node_timing_refresh_tests;
mod rust_format;
mod stdout;
#[cfg(test)]
mod stdout_tests;
pub(crate) mod surface_context;
mod surfaces;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod timing_refresh_policy_tests;

use context::AuditContext;
#[cfg(test)]
pub(crate) use graph::{fixture_version, law_version, schema_version, validator_version};
use receipt::loop_receipt;
pub(crate) use surface_context::validation_surface_context;

use crate::scheduler::{SchedulerConfig, TaskClass};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LiveLoopAction {
    Run,
    Measure,
    FormatCheck,
}

#[derive(Debug)]
pub(crate) struct LiveLoopCommand {
    pub(crate) action: LiveLoopAction,
    pub(crate) tier: String,
    pub(crate) cache_mode: String,
    pub(crate) jobs: Option<usize>,
    pub(crate) receipt: PathBuf,
    pub(crate) node_id: Option<String>,
    pub(crate) measure_all: bool,
}

pub(crate) fn parse(raw: &[String]) -> Result<Option<LiveLoopCommand>, String> {
    match raw {
        [a, b, ..] if a == "loop" && b == "run" => Ok(Some(LiveLoopCommand {
            action: LiveLoopAction::Run,
            tier: opt_string(&raw[2..], "--tier").unwrap_or_else(|| "hot".to_string()),
            cache_mode: opt_string(&raw[2..], "--cache-mode")
                .unwrap_or_else(|| "verified-local".to_string()),
            jobs: opt_jobs(&raw[2..], "--jobs")?,
            receipt: opt_path(&raw[2..], "--receipt").unwrap_or_else(|| {
                PathBuf::from("validation_artifacts/observability/loop-run.json")
            }),
            node_id: None,
            measure_all: false,
        })),
        [a, b, ..] if a == "loop" && b == "measure" => {
            let node_id = opt_string(&raw[2..], "--node");
            let measure_all = has_flag(&raw[2..], "--all");
            if measure_all && node_id.is_some() {
                return Err(
                    "loop measure accepts either --all or --node <id>, not both".to_string()
                );
            }
            if !measure_all && node_id.is_none() {
                return Err("loop measure requires --node <id> or --all".to_string());
            }
            Ok(Some(LiveLoopCommand {
                action: LiveLoopAction::Measure,
                tier: opt_string(&raw[2..], "--tier").unwrap_or_else(|| "hot".to_string()),
                cache_mode: opt_string(&raw[2..], "--cache-mode")
                    .unwrap_or_else(|| "verified-local".to_string()),
                jobs: opt_jobs(&raw[2..], "--jobs")?,
                receipt: opt_path(&raw[2..], "--receipt")
                    .unwrap_or_else(|| PathBuf::from(nodes::timing::NODE_TIMING_REL)),
                node_id,
                measure_all,
            }))
        }
        [a, b, c, ..] if a == "loop" && b == "format" && c == "check" => {
            if !has_flag(&raw[3..], "--changed-rust") {
                return Err("loop format check requires --changed-rust".to_string());
            }
            Ok(Some(LiveLoopCommand {
                action: LiveLoopAction::FormatCheck,
                tier: "hot".to_string(),
                cache_mode: "verified-local".to_string(),
                jobs: None,
                receipt: PathBuf::from("validation_artifacts/observability/loop-format-check.json"),
                node_id: None,
                measure_all: false,
            }))
        }
        _ => Ok(None),
    }
}

pub(crate) fn run(root: &Path, command: &LiveLoopCommand) -> Result<i32, String> {
    if command.action == LiveLoopAction::Measure {
        return nodes::measure(root, command);
    }
    if command.action == LiveLoopAction::FormatCheck {
        return rust_format::run(root);
    }
    let started = Instant::now();
    let candidate = crate::package::inventory::package_digest(root)?;
    let config = SchedulerConfig::from_jobs(command.jobs)?;
    let current_state_path = crate::output_path::literal_claim_artifact_path(
        root,
        "validation_artifacts/current-state.json",
        "current state snapshot",
    );
    crate::output_path::prepare_parent(&current_state_path)?;
    let receipt_path =
        crate::output_path::claim_artifact_path(root, &command.receipt, "live loop receipt")?;
    crate::output_path::prepare_parent(&receipt_path)?;
    let mut snapshot = loop_snapshot(root, &candidate, command, config);
    let timing_refreshes = node_timing_refresh::refresh_hot_repair_timing(
        root,
        command,
        &snapshot.first_blocker,
        snapshot.context.changed_inputs(),
    )?;
    if !timing_refreshes.is_empty() {
        snapshot = loop_snapshot(root, &candidate, command, config);
    }
    snapshot.context.verify_package_truth_current(root)?;
    crate::json_boundary::write_json(&current_state_path, &snapshot.current_state)?;
    let first_blocker = snapshot.first_blocker.clone();
    let status = blockers::status_for_blockers(&snapshot.first_product_blocker, &first_blocker);
    let receipt_result = loop_receipt(
        root,
        command,
        snapshot.context,
        snapshot.scheduled,
        snapshot.current_state,
        first_blocker.clone(),
        snapshot.first_product_blocker,
        snapshot.first_observability_blocker,
        snapshot.first_speed_blocker,
        snapshot.first_control_board_blocker,
        timing_refreshes,
        status,
        started,
    );
    let receipt = receipt_result?;
    crate::json_boundary::write_json(&receipt_path, &receipt)?;
    stdout::print_run_summary(command, &receipt, &first_blocker);
    Ok(i32::from(status != "pass"))
}

struct LoopSnapshot {
    context: AuditContext,
    scheduled: crate::scheduler::Scheduled<Value>,
    current_state: Value,
    first_blocker: Value,
    first_product_blocker: Value,
    first_observability_blocker: Value,
    first_speed_blocker: Value,
    first_control_board_blocker: Value,
}

fn loop_snapshot(
    root: &Path,
    candidate: &str,
    command: &LiveLoopCommand,
    config: SchedulerConfig,
) -> LoopSnapshot {
    let context = AuditContext::new(root, candidate.to_string(), command);
    let scheduled =
        crate::scheduler::run_ordered(config, TaskClass::PureReadParallel, context.tasks());
    let current_state =
        crate::cli::current_state::snapshot_for_candidate(root, candidate.to_string());
    let first_product_blocker = blockers::first_product_blocker(&scheduled.values);
    let first_observability_blocker =
        graph::first_observability_blocker(&scheduled.values).unwrap_or_else(blockers::none);
    let first_speed_blocker =
        graph::first_speed_blocker(&scheduled.values).unwrap_or_else(blockers::none);
    let first_control_board_blocker = blockers::first_control_board_blocker(&current_state);
    let first_blocker = blockers::first_loop_blocker(
        &first_product_blocker,
        &first_observability_blocker,
        &first_speed_blocker,
        &first_control_board_blocker,
    );
    LoopSnapshot {
        context,
        scheduled,
        current_state,
        first_blocker,
        first_product_blocker,
        first_observability_blocker,
        first_speed_blocker,
        first_control_board_blocker,
    }
}

fn opt_string(args: &[String], key: &str) -> Option<String> {
    args.windows(2)
        .find(|window| window[0] == key)
        .map(|window| window[1].clone())
}

fn opt_path(args: &[String], key: &str) -> Option<PathBuf> {
    opt_string(args, key).map(PathBuf::from)
}

fn has_flag(args: &[String], key: &str) -> bool {
    args.iter().any(|arg| arg == key)
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
