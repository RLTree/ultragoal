mod cache_replay;
#[cfg(test)]
mod command;
#[cfg(test)]
mod flow_tests;
mod full_command;
#[cfg(test)]
mod high_frequency_nodes;
mod observation;
#[cfg(test)]
mod runtime_command_tests;
mod surface_selection;
mod timing;

use super::super::LiveLoopCommand;
use super::super::changed_inputs::ChangedInputs;
use super::super::graph;
use super::super::surfaces::LoopValidationSurface;
use full_command::{run_full_command, run_narrow_command};
use std::path::Path;
use std::time::Instant;
use surface_selection::selected_surfaces;
use timing::receipt::{affected_set_status, print_measurements, write_node_timings};
use timing::{record::NodeTimingRow, record::node_timing_row, verified_work::VerifiedLocalProof};

pub(crate) fn measure(root: &Path, command: &LiveLoopCommand) -> Result<i32, String> {
    let surfaces = selected_surfaces(command)?;
    measure_surfaces(root, command, surfaces)
}

pub(crate) fn measure_surfaces(
    root: &Path,
    command: &LiveLoopCommand,
    surfaces: Vec<LoopValidationSurface>,
) -> Result<i32, String> {
    let candidate = crate::package::inventory::package_digest(root)?;
    let inputs = ChangedInputs::collect(root, &candidate, &command.tier, &command.cache_mode);
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
            affected_set_status(inputs.changed_file_count),
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
    affected_set_status: &'static str,
) -> NodeTimingRow {
    let input_digest = graph::surface_input_digest(
        surface,
        candidate,
        inputs.surface_digest(surface),
        &inputs.audit_context_digest,
    );
    if let Some((baseline, verified_local)) =
        measure_cached_verified_local(root, surface, candidate, &input_digest, command)
    {
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
        let verified_local =
            measure_executed_verified_local(root, surface, candidate, &input_digest, command);
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
    let verified_local =
        measure_executed_verified_local(root, surface, candidate, &input_digest, command);
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

fn measure_cached_verified_local(
    root: &Path,
    surface: LoopValidationSurface,
    candidate: &str,
    input_digest: &str,
    command: &LiveLoopCommand,
) -> Option<(full_command::FullCommandRun, VerifiedLocalProof)> {
    let started = Instant::now();
    let cache_key = graph::verified_local_cache_key(
        surface.id,
        input_digest,
        &command.tier,
        &command.cache_mode,
    );
    let graph_overhead_ms = elapsed_ms(started);
    let replay_started = Instant::now();
    if let Some(mut actual_work) = cache_replay::verified_local_hit(
        root,
        surface,
        candidate,
        input_digest,
        command,
        &cache_key,
        replay_started,
    ) {
        let telemetry_reconciliation = actual_work.telemetry_reconciliation.clone();
        actual_work.run.failure = telemetry_reconciliation.failure_summary();
        let proof = VerifiedLocalProof {
            proof_kind: "verified_cache_hit",
            cache_hit: true,
            cache_key,
            graph_overhead_ms,
            actual_work: actual_work.run,
            work_unit_count: 0,
            equivalence_status: "verified_same_candidate_cache_replay".to_string(),
            invalidation_proof: actual_work.invalidation_proof,
            telemetry_reconciliation_status: telemetry_reconciliation.status.clone(),
            telemetry_reconciliation_duration_ms: telemetry_reconciliation.duration_ms,
            telemetry_reconciliation,
            prior_result_digest: Some(actual_work.prior_result_digest),
            replayed_output_digest: Some(actual_work.replayed_output_digest),
            cache_equivalence_status: Some("pass".to_string()),
        };
        return Some((actual_work.baseline, proof));
    }
    None
}

fn measure_executed_verified_local(
    root: &Path,
    surface: LoopValidationSurface,
    candidate: &str,
    input_digest: &str,
    command: &LiveLoopCommand,
) -> VerifiedLocalProof {
    let started = Instant::now();
    let cache_key = graph::verified_local_cache_key(
        surface.id,
        input_digest,
        &command.tier,
        &command.cache_mode,
    );
    let graph_overhead_ms = elapsed_ms(started);
    let mut actual_work = run_narrow_command(root, surface);
    let telemetry_reconciliation = if hot_validation_retains_observability(command) {
        observation::deferred_hot_validation(surface)
    } else {
        observation::reconcile(root, surface, candidate, command, &actual_work)
    };
    if telemetry_reconciliation.status != "deferred_hot_loop_observability" {
        actual_work.failure = telemetry_reconciliation.failure_summary();
    }
    VerifiedLocalProof {
        proof_kind: "executed",
        cache_hit: false,
        cache_key,
        graph_overhead_ms,
        actual_work,
        work_unit_count: 1,
        equivalence_status: "executed_current_candidate_not_cache_replay".to_string(),
        invalidation_proof: "cache_not_used_current_command_executed".to_string(),
        telemetry_reconciliation_status: telemetry_reconciliation.status.clone(),
        telemetry_reconciliation_duration_ms: telemetry_reconciliation.duration_ms,
        telemetry_reconciliation,
        prior_result_digest: None,
        replayed_output_digest: None,
        cache_equivalence_status: None,
    }
}

fn hot_validation_retains_observability(command: &LiveLoopCommand) -> bool {
    command.tier == "hot" && command.cache_mode == "verified-local"
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis())
        .unwrap_or(u64::MAX)
        .max(1)
}
