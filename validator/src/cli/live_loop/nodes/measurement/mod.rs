#[cfg(test)]
mod command_surface;
mod full_command;
#[cfg(test)]
mod high_frequency_nodes;
mod node_timing_receipt;

use super::super::LiveLoopCommand;
use super::super::graph;
use super::super::surfaces::{LOOP_VALIDATION_SURFACES, LoopValidationSurface, surface_by_id};
use full_command::run_full_command;
use node_timing_receipt::{
    affected_set_status, node_timing_row, print_measurements, write_node_timings,
};
use serde_json::Value;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

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
    let verified_local = measure_verified_local(surface, &input_digest, command);
    node_timing_row(
        surface,
        command,
        candidate,
        changed_files_digest,
        audit_context_digest,
        &input_digest,
        &baseline,
        verified_local,
        affected_set_status,
    )
}

fn measure_verified_local(
    surface: LoopValidationSurface,
    input_digest: &str,
    command: &LiveLoopCommand,
) -> u64 {
    let started = Instant::now();
    let _ = graph::verified_local_cache_key(
        surface.id,
        input_digest,
        &command.tier,
        &command.cache_mode,
    );
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
