use super::{LiveLoopAction, LiveLoopCommand, changed_inputs::ChangedInputs, nodes, surfaces};
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
    refresh_batch_exit_code: i32,
    exit_code_scope: &'static str,
    status: &'static str,
    claim_impact: &'static str,
}

pub(super) fn refresh_hot_repair_timing(
    root: &Path,
    command: &LiveLoopCommand,
    blocker: &Value,
    changed_inputs: &ChangedInputs,
) -> Result<Vec<TimingRefresh>, String> {
    if command.cache_mode != "verified-local" {
        return Ok(Vec::new());
    }
    let failure_class = text(blocker, "failure_class", "");
    if !refreshable_failure_class(failure_class) {
        return Ok(Vec::new());
    }
    let refresh_surfaces = refresh_surfaces(blocker, changed_inputs);
    if refresh_surfaces.is_empty() {
        return Ok(Vec::new());
    }
    refresh_surface_timing(root, command, &refresh_surfaces, failure_class)
}

pub(super) fn refresh_surfaces(
    blocker: &Value,
    changed_inputs: &ChangedInputs,
) -> Vec<surfaces::LoopValidationSurface> {
    let affected = changed_inputs.affected_high_frequency_surfaces();
    if !affected.is_empty() {
        return affected;
    }
    let blocker_id = text(blocker, "id", "");
    surfaces::surface_by_id(blocker_id)
        .filter(|surface| surface.high_frequency)
        .into_iter()
        .collect()
}

#[cfg(test)]
pub(super) fn should_refresh_surface(
    surface: &surfaces::LoopValidationSurface,
    blocker: &Value,
    changed_inputs: &ChangedInputs,
) -> bool {
    if !surface.high_frequency {
        return false;
    }
    let failure_class = text(blocker, "failure_class", "");
    failure_class == "live_loop_high_frequency_measurement_missing"
        && (changed_inputs.affects_surface(*surface)
            || changed_inputs.affected_high_frequency_surfaces().is_empty()
                && text(blocker, "id", "") == surface.id)
}

fn refresh_surface_timing(
    root: &Path,
    command: &LiveLoopCommand,
    refresh_surfaces: &[surfaces::LoopValidationSurface],
    failure_class: &str,
) -> Result<Vec<TimingRefresh>, String> {
    let measure_command = LiveLoopCommand {
        action: LiveLoopAction::Measure,
        tier: command.tier.clone(),
        cache_mode: command.cache_mode.clone(),
        jobs: command.jobs,
        receipt: nodes::timing::NODE_TIMING_REL.into(),
        node_id: None,
        measure_all: false,
    };
    let exit_code = nodes::measure_surfaces(root, &measure_command, refresh_surfaces.to_vec())?;
    Ok(refresh_surfaces
        .iter()
        .map(|surface| TimingRefresh {
            node_id: surface.id,
            surface: surface.surface,
            trigger_failure_class: failure_class.to_string(),
            task_class: TaskClass::SharedAuthorityWriteSerial.id(),
            serial_reason: "live_loop_run_refreshes_affected_node_timing_receipt_before_parallel_read_graph",
            command: format!(
                "target/debug/ultragoal --root . loop measure --node {} --tier {} --cache-mode {}",
                surface.id, command.tier, command.cache_mode
            ),
            receipt: nodes::timing::NODE_TIMING_REL,
            refresh_batch_exit_code: exit_code,
            exit_code_scope: "aggregate_for_refresh_batch_not_per_node",
            status: "measurement_batch_result_recorded",
            claim_impact: "source_local_live_loop_measurement_only_no_readiness_release_completion_update_goal",
        })
        .collect())
}

fn text<'a>(value: &'a Value, key: &str, default: &'a str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or(default)
}

pub(super) fn refreshable_failure_class(failure_class: &str) -> bool {
    matches!(
        failure_class,
        "live_loop_high_frequency_measurement_missing"
    )
}
