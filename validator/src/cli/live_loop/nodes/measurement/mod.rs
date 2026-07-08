mod cache_replay;
#[cfg(test)]
mod command;
#[cfg(test)]
mod flow_tests;
mod full_command;
#[cfg(test)]
mod high_frequency_nodes;
mod observation;
mod observation_mode;
#[cfg(test)]
mod runtime_command_tests;
#[cfg(test)]
mod snapshot_flow_tests;
mod surface_selection;
mod timing;
mod verified_local_work;

use super::super::LiveLoopCommand;
use super::super::changed_inputs::ChangedInputs;
use super::super::graph;
use super::super::surfaces::LoopValidationSurface;
use full_command::run_full_command;
pub(crate) use observation_mode::ObservationMode;
use std::path::Path;
use surface_selection::selected_surfaces;
use timing::receipt::{affected_set_status, print_measurements, write_node_timings};
use timing::{record::NodeTimingRow, record::node_timing_row};
use verified_local_work::{cached_verified_local, executed_verified_local};

pub(crate) fn measure(root: &Path, command: &LiveLoopCommand) -> Result<i32, String> {
    let surfaces = selected_surfaces(command)?;
    measure_surfaces(root, command, surfaces)
}

pub(crate) fn measure_surfaces(
    root: &Path,
    command: &LiveLoopCommand,
    surfaces: Vec<LoopValidationSurface>,
) -> Result<i32, String> {
    measure_surfaces_with_observation(root, command, surfaces, ObservationMode::FullRoundtrip)
}

pub(crate) fn measure_surfaces_with_observation(
    root: &Path,
    command: &LiveLoopCommand,
    surfaces: Vec<LoopValidationSurface>,
    observation_mode: ObservationMode,
) -> Result<i32, String> {
    let candidate = crate::package::inventory::package_digest(root)?;
    let inputs = ChangedInputs::collect(root, &candidate, &command.tier, &command.cache_mode);
    let replay_store = cache_replay::ReplayStore::load(root, command);
    let mut rows = Vec::new();
    for surface in surfaces {
        println!(
            "ultragoal-loop-measure-start candidate={} node={} command='{}' claim_ceiling='source-local loop timing only'",
            candidate, surface.id, surface.canonical_full_command
        );
        let row = measure_surface(
            root,
            command,
            surface,
            &candidate,
            &inputs,
            &replay_store,
            affected_set_status(inputs.changed_file_count),
            observation_mode,
        );
        print_measurements(command, &candidate, std::slice::from_ref(&row));
        rows.push(row);
    }
    let exit_code = i32::from(rows.iter().any(NodeTimingRow::blocks_hot_loop));
    write_node_timings(
        root,
        &command.receipt,
        &candidate,
        &command.tier,
        &command.cache_mode,
        rows.clone(),
    )?;
    Ok(exit_code)
}

fn measure_surface(
    root: &Path,
    command: &LiveLoopCommand,
    surface: LoopValidationSurface,
    candidate: &str,
    inputs: &ChangedInputs,
    replay_store: &cache_replay::ReplayStore,
    affected_set_status: &'static str,
    observation_mode: ObservationMode,
) -> NodeTimingRow {
    let input_digest = graph::surface_input_digest(
        surface,
        candidate,
        inputs.surface_digest(surface),
        &inputs.audit_context_digest,
    );
    if let Some((baseline, verified_local)) = cached_verified_local(
        replay_store,
        surface,
        candidate,
        &input_digest,
        command,
        observation_mode,
    ) {
        return node_timing_row(
            surface,
            command,
            candidate,
            &inputs.changed_files_digest,
            &inputs.audit_context_digest,
            &input_digest,
            &baseline,
            &verified_local,
            affected_set_status,
        );
    }
    if same_command_baseline_reuse_allowed(surface) {
        let verified_local = executed_verified_local(
            root,
            surface,
            candidate,
            &input_digest,
            command,
            observation_mode,
        );
        let baseline = verified_local.actual_work.clone();
        return node_timing_row(
            surface,
            command,
            candidate,
            &inputs.changed_files_digest,
            &inputs.audit_context_digest,
            &input_digest,
            &baseline,
            &verified_local,
            affected_set_status,
        );
    }
    let baseline = run_full_command(root, surface);
    let verified_local = executed_verified_local(
        root,
        surface,
        candidate,
        &input_digest,
        command,
        observation_mode,
    );
    node_timing_row(
        surface,
        command,
        candidate,
        &inputs.changed_files_digest,
        &inputs.audit_context_digest,
        &input_digest,
        &baseline,
        &verified_local,
        affected_set_status,
    )
}

fn same_command_baseline_reuse_allowed(surface: LoopValidationSurface) -> bool {
    surface.canonical_full_command.trim() == surface.narrow_rerun.trim()
}
