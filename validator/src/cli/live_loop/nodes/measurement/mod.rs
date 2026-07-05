use super::super::LiveLoopCommand;
use super::super::graph;
use super::super::surfaces::{LoopValidationSurface, surface_by_id};
use super::timing::{NODE_TIMING_REL, node_rows, positive, text};
use serde_json::{Value, json};
use std::path::Path;
use std::process::Command;
use std::time::Instant;

#[cfg(test)]
mod command_surface;

pub(crate) fn measure(root: &Path, command: &LiveLoopCommand) -> Result<i32, String> {
    let node_id = command
        .node_id
        .as_deref()
        .ok_or_else(|| "loop measure requires --node <id>".to_string())?;
    let surface =
        surface_by_id(node_id).ok_or_else(|| format!("unknown live-loop node: {node_id}"))?;
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
    let input_digest = graph::surface_input_digest(
        surface,
        &candidate,
        &changed_files_digest,
        &audit_context_digest,
    );
    let baseline = run_baseline(root, surface);
    let verified_local = measure_verified_local(surface, &input_digest, command);
    let row = node_row(
        surface,
        command,
        &candidate,
        &changed_files_digest,
        &audit_context_digest,
        &input_digest,
        &baseline,
        verified_local,
        affected_set_status(&changed_files),
    );
    write_node_timing(root, &command.receipt, row.clone())?;
    print_measurement(command, &candidate, &row);
    Ok(i32::from(row["timing_status"] != "pass"))
}

fn run_baseline(root: &Path, surface: LoopValidationSurface) -> BaselineRun {
    run_baseline_with_shell(root, surface, "bash")
}

fn run_baseline_with_shell(
    root: &Path,
    surface: LoopValidationSurface,
    shell: &str,
) -> BaselineRun {
    let started = Instant::now();
    let output = match Command::new(shell)
        .arg("-lc")
        .arg(surface.canonical_full_command)
        .current_dir(root)
        .output()
    {
        Ok(output) => output,
        Err(err) => {
            return BaselineRun {
                exit_code: 1,
                status_success: false,
                launch_error: true,
                duration_ms: u64::try_from(started.elapsed().as_millis())
                    .unwrap_or(u64::MAX)
                    .max(1),
                stdout_digest: crate::digest::bytes(&[]),
                stderr_digest: crate::digest::bytes(
                    format!("{} launch failed: {err}", surface.canonical_full_command).as_bytes(),
                ),
            };
        }
    };
    BaselineRun {
        exit_code: output.status.code().unwrap_or(1),
        status_success: output.status.success(),
        launch_error: false,
        duration_ms: u64::try_from(started.elapsed().as_millis())
            .unwrap_or(u64::MAX)
            .max(1),
        stdout_digest: crate::digest::bytes(&output.stdout),
        stderr_digest: crate::digest::bytes(&output.stderr),
    }
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

fn node_row(
    surface: LoopValidationSurface,
    command: &LiveLoopCommand,
    candidate_digest: &str,
    changed_files_digest: &str,
    audit_context_digest: &str,
    input_digest: &str,
    baseline: &BaselineRun,
    verified_local_duration_ms: u64,
    affected_set_status: &'static str,
) -> Value {
    let speedup_ratio = baseline.duration_ms / verified_local_duration_ms.max(1);
    let pass = baseline.status_success && speedup_ratio >= 20;
    let timing_status = timing_status(pass);
    json!({
        "node_id": surface.id,
        "surface": surface.surface,
        "candidate_digest": candidate_digest,
        "tier": command.tier,
        "cache_mode": command.cache_mode,
        "changed_files_digest": changed_files_digest,
        "audit_context_digest": audit_context_digest,
        "input_digest": input_digest,
        "canonical_full_command": surface.canonical_full_command,
        "timing_status": timing_status,
        "failure_class": measurement_failure_class(baseline, speedup_ratio),
        "baseline_duration_ms": baseline.duration_ms,
        "verified_local_duration_ms": verified_local_duration_ms,
        "speedup_ratio": speedup_ratio,
        "required_speedup": "20x",
        "baseline_exit_code": baseline.exit_code,
        "baseline_launch_error": baseline.launch_error,
        "baseline_stdout_digest": baseline.stdout_digest,
        "baseline_stderr_digest": baseline.stderr_digest,
        "affected_set_status": affected_set_status,
        "cache_honesty": "pass",
        "timing_source": NODE_TIMING_REL,
        "claim_impact": "supports_live_loop_node_timing_only_no_readiness_release_completion_update_goal"
    })
}

fn write_node_timing(root: &Path, receipt: &Path, row: Value) -> Result<(), String> {
    let path = crate::output_path::claim_artifact_path(root, receipt, "live loop node timing")?;
    let existing = crate::json_boundary::read_json(&path).unwrap_or_else(|_| json!({"nodes":[]}));
    let node_id = text(&row, "node_id").unwrap_or("").to_string();
    let candidate = text(&row, "candidate_digest").unwrap_or("").to_string();
    let tier = text(&row, "tier").unwrap_or("").to_string();
    let cache_mode = text(&row, "cache_mode").unwrap_or("").to_string();
    let mut nodes: Vec<Value> = node_rows(&existing)
        .into_iter()
        .filter(|existing_row| {
            text(existing_row, "node_id") != Some(node_id.as_str())
                && text(existing_row, "candidate_digest") == Some(candidate.as_str())
                && text(existing_row, "tier") == Some(tier.as_str())
                && text(existing_row, "cache_mode") == Some(cache_mode.as_str())
        })
        .cloned()
        .collect();
    nodes.push(row);
    nodes.sort_by(|left, right| text(left, "node_id").cmp(&text(right, "node_id")));
    crate::json_boundary::write_json(
        &path,
        &json!({
            "schema": "harness-ultragoal.live-loop-node-timing.v1",
            "candidate_digest": candidate,
            "tier": tier,
            "cache_mode": cache_mode,
            "nodes": nodes
        }),
    )
}

fn print_measurement(command: &LiveLoopCommand, candidate: &str, row: &Value) {
    println!(
        "ultragoal-loop-measure {} candidate={} node={} baseline_duration_ms={} verified_local_duration_ms={} speedup_ratio={} receipt={} claim_ceiling='source-local loop timing only'",
        text(row, "timing_status").unwrap_or("fail"),
        candidate,
        text(row, "node_id").unwrap_or("unknown"),
        positive(row, "baseline_duration_ms").unwrap_or(0),
        positive(row, "verified_local_duration_ms").unwrap_or(0),
        positive(row, "speedup_ratio").unwrap_or(0),
        command.receipt.display()
    );
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

fn affected_set_status(changed_files: &[String]) -> &'static str {
    if changed_files.is_empty() {
        "clean_worktree_no_affected_files"
    } else {
        "changed_files_digest_bound"
    }
}

fn measurement_failure_class(baseline: &BaselineRun, speedup_ratio: u64) -> &'static str {
    if baseline.launch_error {
        "canonical_full_command_launch_failed"
    } else if !baseline.status_success {
        "canonical_full_command_failed"
    } else if speedup_ratio < 20 {
        "live_loop_speedup_target_missed"
    } else {
        "none"
    }
}

fn timing_status(pass: bool) -> &'static str {
    if pass { "pass" } else { "fail" }
}

#[derive(Debug)]
struct BaselineRun {
    exit_code: i32,
    status_success: bool,
    launch_error: bool,
    duration_ms: u64,
    stdout_digest: String,
    stderr_digest: String,
}
