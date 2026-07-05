#[cfg(test)]
mod command;
mod full_command;
#[cfg(test)]
mod high_frequency_nodes;
mod timing;

use super::super::LiveLoopCommand;
use super::super::graph;
use super::super::surfaces::{LOOP_VALIDATION_SURFACES, LoopValidationSurface, surface_by_id};
use full_command::{FullCommandRun, run_full_command, run_narrow_command};
use serde_json::Value;
use std::path::Path;
use std::process::Command;
use std::time::Instant;
use timing::receipt::{affected_set_status, print_measurements, write_node_timings};
use timing::record::{VerifiedLocalProof, node_timing_row};

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
    let baseline = run_full_command(root, surface);
    let verified_local = measure_verified_local(root, surface, &input_digest, command);
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

fn measure_verified_local(
    root: &Path,
    surface: LoopValidationSurface,
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
    let actual_work = run_narrow_command(root, surface);
    let telemetry_reconciliation_status = telemetry_reconciliation_status(&actual_work);
    VerifiedLocalProof {
        proof_kind: "executed",
        cache_hit: false,
        cache_key,
        graph_overhead_ms,
        actual_work,
        work_unit_count: 1,
        equivalence_status: "executed_current_candidate_not_cache_replay",
        invalidation_proof: "cache_not_used_current_command_executed",
        telemetry_reconciliation_status,
    }
}

fn telemetry_reconciliation_status(run: &FullCommandRun) -> &'static str {
    if run.failure.run_id.is_none() || run.failure.correlation_id.is_none() {
        "missing_command_telemetry"
    } else {
        "missing_query_reconciliation"
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

#[cfg(test)]
mod telemetry_status_tests {
    use super::full_command::FullCommandRun;
    use super::telemetry_reconciliation_status;
    use crate::cli::live_loop::nodes::command_failure::CommandFailureSummary;

    #[test]
    fn telemetry_reconciliation_requires_query_after_command_ids_exist() {
        let run = FullCommandRun {
            exit_code: 1,
            status_success: false,
            launch_error: false,
            duration_ms: 1,
            stdout_digest: "sha256:stdout".to_string(),
            stderr_digest: "sha256:stderr".to_string(),
            failure: CommandFailureSummary {
                run_id: Some("run-current".to_string()),
                correlation_id: Some("corr-current".to_string()),
                ..Default::default()
            },
        };

        assert_eq!(
            telemetry_reconciliation_status(&run),
            "missing_query_reconciliation"
        );
    }
}
