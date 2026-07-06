#[cfg(test)]
mod command_registry_tests;
mod context;
mod graph;
mod node_timing_refresh;
mod nodes;
#[cfg(test)]
mod parser_tests;
mod receipt;
#[cfg(test)]
mod run_node_timing_refresh_tests;
mod stdout;
#[cfg(test)]
mod stdout_tests;
mod surfaces;
#[cfg(test)]
mod tests;

use context::AuditContext;
use receipt::loop_receipt;

use crate::scheduler::{SchedulerConfig, TaskClass};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LiveLoopAction {
    Run,
    Measure,
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
        _ => Ok(None),
    }
}

pub(crate) fn run(root: &Path, command: &LiveLoopCommand) -> Result<i32, String> {
    if command.action == LiveLoopAction::Measure {
        return nodes::measure(root, command);
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
    let mut timing_refreshes = Vec::new();
    if let Some(measurement) =
        node_timing_refresh::refresh_current_blocker_timing(root, command, &snapshot.first_blocker)?
    {
        timing_refreshes.push(measurement);
        snapshot = loop_snapshot(root, &candidate, command, config);
    }
    crate::json_boundary::write_json(&current_state_path, &snapshot.current_state)?;
    let first_blocker = snapshot.first_blocker.clone();
    let status = status_for_blocker(&first_blocker);
    let receipt_result = loop_receipt(
        root,
        command,
        snapshot.context,
        snapshot.scheduled,
        snapshot.current_state,
        first_blocker.clone(),
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
    let first_blocker = first_loop_blocker(&scheduled.values, &current_state);
    LoopSnapshot {
        context,
        scheduled,
        current_state,
        first_blocker,
    }
}

fn status_for_blocker(blocker: &Value) -> &'static str {
    if blocker.get("id").and_then(Value::as_str) == Some("none") {
        "pass"
    } else {
        "fail"
    }
}

fn first_loop_blocker(nodes: &[Value], current_state: &Value) -> Value {
    graph::first_blocker(nodes).unwrap_or_else(|| current_state["first_blocker"].clone())
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

#[cfg(test)]
mod projection_tests {
    use serde_json::json;

    #[test]
    fn loop_status_projects_current_blocker_state() {
        assert_eq!(super::status_for_blocker(&json!({"id": "none"})), "pass");
        assert_eq!(
            super::status_for_blocker(&json!({"id": "fmt_check"})),
            "fail"
        );
    }

    #[test]
    fn loop_blocker_uses_current_state_when_graph_has_no_blocker() {
        let current_state = json!({
            "first_blocker": {
                "id": "coverage_prove",
                "why_failed": "coverage receipt is stale"
            }
        });
        let blocker = super::first_loop_blocker(&[], &current_state);
        assert_eq!(blocker["id"], "coverage_prove");
        assert_eq!(blocker["why_failed"], "coverage receipt is stale");
    }
}
