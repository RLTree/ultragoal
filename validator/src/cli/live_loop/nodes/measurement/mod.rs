mod cache_replay;
#[cfg(test)]
mod command;
mod full_command;
#[cfg(test)]
mod high_frequency_nodes;
mod observation;
mod timing;

use super::super::LiveLoopCommand;
use super::super::graph;
use super::super::surfaces::{LOOP_VALIDATION_SURFACES, LoopValidationSurface, surface_by_id};
use full_command::{run_full_command, run_narrow_command};
use serde_json::Value;
use std::path::Path;
use std::process::Command;
use std::time::Instant;
use timing::receipt::{affected_set_status, print_measurements, write_node_timings};
use timing::{record::node_timing_row, verified_work::VerifiedLocalProof};

pub(crate) fn measure(root: &Path, command: &LiveLoopCommand) -> Result<i32, String> {
    let surfaces = selected_surfaces(command)?;
    let candidate = crate::package::inventory::package_digest(root)?;
    let changed_files = changed_files(root);
    let changed_files_digest = crate::digest::bytes(changed_files.join("\n").as_bytes());
    let audit_context_digest = crate::digest::bytes(
        format!(
            "{}:{}:{}:{}",
            candidate, command.tier, command.cache_mode, changed_files_digest
        )
        .as_bytes(),
    );
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
            &changed_files_digest,
            &audit_context_digest,
            affected_set_status(&changed_files),
        );
        print_measurements(command, &candidate, std::slice::from_ref(&row));
        rows.push(row);
    }
    write_node_timings(
        root,
        &command.receipt,
        &candidate,
        &command.tier,
        &command.cache_mode,
        rows.clone(),
    )?;
    Ok(i32::from(
        rows.iter().any(|row| row["timing_status"] != "pass"),
    ))
}

fn selected_surfaces(command: &LiveLoopCommand) -> Result<Vec<LoopValidationSurface>, String> {
    if command.measure_all {
        return high_frequency_surfaces_from(LOOP_VALIDATION_SURFACES);
    }
    let node_id = command
        .node_id
        .as_deref()
        .ok_or_else(|| "loop measure requires --node <id> or --all".to_string())?;
    surface_by_id(node_id)
        .map(|surface| vec![surface])
        .ok_or_else(|| format!("unknown live-loop node: {node_id}"))
}

fn high_frequency_surfaces_from(
    surfaces: &[LoopValidationSurface],
) -> Result<Vec<LoopValidationSurface>, String> {
    let selected: Vec<LoopValidationSurface> = surfaces
        .iter()
        .copied()
        .filter(|surface| surface.high_frequency)
        .collect();
    if selected.is_empty() {
        Err("live-loop high-frequency registry has no command surfaces".to_string())
    } else {
        Ok(selected)
    }
}

fn measure_surface(
    root: &Path,
    command: &LiveLoopCommand,
    surface: LoopValidationSurface,
    candidate: &str,
    changed_files_digest: &str,
    audit_context_digest: &str,
    affected_set_status: &'static str,
) -> Value {
    let input_digest = graph::surface_input_digest(
        surface,
        candidate,
        changed_files_digest,
        audit_context_digest,
    );
    if let Some((baseline, verified_local)) =
        measure_cached_verified_local(root, surface, candidate, &input_digest, command)
    {
        return node_timing_row(
            surface,
            command,
            candidate,
            changed_files_digest,
            audit_context_digest,
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
        changed_files_digest,
        audit_context_digest,
        &input_digest,
        &baseline,
        &verified_local,
        affected_set_status,
    )
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
        let telemetry_reconciliation =
            observation::reconcile(root, surface, candidate, command, &actual_work.run);
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
    let telemetry_reconciliation =
        observation::reconcile(root, surface, candidate, command, &actual_work);
    actual_work.failure = telemetry_reconciliation.failure_summary();
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

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis())
        .unwrap_or(u64::MAX)
        .max(1)
}

fn changed_files(root: &Path) -> Vec<String> {
    let output = Command::new("git")
        .args(["status", "--short", "--untracked-files=all"])
        .current_dir(root)
        .output();
    output
        .ok()
        .map(|out| {
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}
