use super::{LiveLoopAction, LiveLoopCommand, nodes, surfaces};
use crate::scheduler::TaskClass;
use serde::Serialize;
use serde_json::Value;
use std::path::Path;

#[derive(Clone, Debug, Serialize)]
pub(super) struct TimingRefresh {
    node_id: &'static str,
    surface: &'static str,
    trigger_failure_class: String,
    task_class: &'static str,
    serial_reason: &'static str,
    command: String,
    receipt: &'static str,
    exit_code: i32,
    status: &'static str,
    claim_impact: &'static str,
}

pub(super) fn refresh_current_blocker_timing(
    root: &Path,
    command: &LiveLoopCommand,
    blocker: &Value,
) -> Result<Option<TimingRefresh>, String> {
    if command.cache_mode != "verified-local" {
        return Ok(None);
    }
    let node_id = text(blocker, "id", "");
    let Some(surface) = surfaces::surface_by_id(node_id).filter(|surface| surface.high_frequency)
    else {
        return Ok(None);
    };
    let failure_class = text(blocker, "failure_class", "");
    if !matches!(
        failure_class,
        "live_loop_high_frequency_measurement_missing" | "live_loop_speedup_target_missed"
    ) {
        return Ok(None);
    }
    let measure_command = LiveLoopCommand {
        action: LiveLoopAction::Measure,
        tier: command.tier.clone(),
        cache_mode: command.cache_mode.clone(),
        jobs: command.jobs,
        receipt: nodes::timing::NODE_TIMING_REL.into(),
        node_id: Some(surface.id.to_string()),
        measure_all: false,
    };
    let exit_code = nodes::measure(root, &measure_command)?;
    Ok(Some(TimingRefresh {
        node_id: surface.id,
        surface: surface.surface,
        trigger_failure_class: failure_class.to_string(),
        task_class: TaskClass::SharedAuthorityWriteSerial.id(),
        serial_reason: "live_loop_run_refreshes_canonical_node_timing_receipt_before_parallel_read_graph",
        command: format!(
            "target/debug/ultragoal --root . loop measure --node {} --tier {} --cache-mode {}",
            surface.id, command.tier, command.cache_mode
        ),
        receipt: nodes::timing::NODE_TIMING_REL,
        exit_code,
        status: "measurement_command_result_recorded",
        claim_impact: "source_local_live_loop_measurement_only_no_readiness_release_completion_update_goal",
    }))
}

fn text<'a>(value: &'a Value, key: &str, default: &'a str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn node_timing_refresh_requires_verified_local_cache_mode() {
        let command = command("none");
        let blocker = json!({
            "id": "fmt_check",
            "failure_class": "live_loop_high_frequency_measurement_missing"
        });
        let refresh = refresh_current_blocker_timing(Path::new("."), &command, &blocker)
            .expect("refresh policy check");
        assert!(refresh.is_none());
    }

    #[test]
    fn node_timing_refresh_ignores_non_high_frequency_blockers() {
        let command = command("verified-local");
        let blocker = json!({
            "id": "observability_control_board",
            "failure_class": "live_loop_high_frequency_measurement_missing"
        });
        let refresh = refresh_current_blocker_timing(Path::new("."), &command, &blocker)
            .expect("refresh policy check");
        assert!(refresh.is_none());
    }

    #[test]
    fn node_timing_refresh_ignores_non_timing_failure_classes() {
        let command = command("verified-local");
        let blocker = json!({
            "id": "fmt_check",
            "failure_class": "live_loop_telemetry_reconciliation_missing"
        });
        let refresh = refresh_current_blocker_timing(Path::new("."), &command, &blocker)
            .expect("refresh policy check");
        assert!(refresh.is_none());
    }

    fn command(cache_mode: &str) -> LiveLoopCommand {
        LiveLoopCommand {
            action: LiveLoopAction::Run,
            tier: "hot".to_string(),
            cache_mode: cache_mode.to_string(),
            jobs: Some(2),
            receipt: "validation_artifacts/observability/loop-test.json".into(),
            node_id: None,
            measure_all: false,
        }
    }
}
