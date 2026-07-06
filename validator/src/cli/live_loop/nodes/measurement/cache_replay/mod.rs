use super::super::timing::NODE_TIMING_REL;
use super::full_command::FullCommandRun;
use crate::cli::live_loop::{LiveLoopCommand, surfaces::LoopValidationSurface};
use serde_json::Value;
use std::path::Path;
use std::time::Instant;

pub(super) struct CacheReplay {
    pub(super) run: FullCommandRun,
    pub(super) prior_result_digest: String,
    pub(super) replayed_output_digest: String,
    pub(super) invalidation_proof: String,
}

pub(super) fn verified_local_hit(
    root: &Path,
    surface: LoopValidationSurface,
    candidate: &str,
    input_digest: &str,
    command: &LiveLoopCommand,
    cache_key: &str,
    started: Instant,
) -> Option<CacheReplay> {
    if command.cache_mode != "verified-local" {
        return None;
    }
    let value = crate::json_boundary::read_json(&root.join(NODE_TIMING_REL)).ok()?;
    node_rows(&value).into_iter().find_map(|row| {
        replay_from_row(
            row,
            surface,
            candidate,
            input_digest,
            command,
            cache_key,
            started,
        )
    })
}

fn replay_from_row(
    row: &Value,
    surface: LoopValidationSurface,
    candidate: &str,
    input_digest: &str,
    command: &LiveLoopCommand,
    cache_key: &str,
    started: Instant,
) -> Option<CacheReplay> {
    if text(row, "node_id")? != surface.id
        || text(row, "candidate_digest")? != candidate
        || text(row, "tier")? != command.tier
        || text(row, "cache_mode")? != command.cache_mode
        || text(row, "input_digest")? != input_digest
        || text(row, "current_input_digest")? != input_digest
        || text(row, "canonical_full_command")? != surface.canonical_full_command
        || text(row, "proof_kind")? != "executed"
        || text(row, "cache_key")? != cache_key
        || text(row, "cache_honesty")? != "pass"
        || text(row, "telemetry_reconciliation_status")? != "pass"
    {
        return None;
    }
    let exit_code = row
        .get("verified_local_exit_code")?
        .as_i64()
        .and_then(|value| i32::try_from(value).ok())?;
    let launch_error = row.get("verified_local_launch_error")?.as_bool()?;
    if launch_error || exit_code != 0 {
        return None;
    }
    let stdout_digest = valid_digest(text(row, "verified_local_stdout_digest")?)?;
    let stderr_digest = valid_digest(text(row, "verified_local_stderr_digest")?)?;
    let replayed_output_digest =
        crate::digest::bytes(format!("stdout={stdout_digest};stderr={stderr_digest}").as_bytes());
    if text(row, "output_digest")? != replayed_output_digest
        || text(row, "verified_local_output_digest")? != replayed_output_digest
    {
        return None;
    }
    let prior_result_digest = valid_digest(text(row, "result_digest")?)?;
    if text(row, "verified_local_result_digest")? != prior_result_digest {
        return None;
    }
    Some(CacheReplay {
        run: FullCommandRun {
            exit_code,
            status_success: true,
            launch_error: false,
            duration_ms: elapsed_ms(started),
            stdout_digest: stdout_digest.to_string(),
            stderr_digest: stderr_digest.to_string(),
            failure: Default::default(),
        },
        prior_result_digest: prior_result_digest.to_string(),
        replayed_output_digest,
        invalidation_proof:
            "cache_key_current_input_digest_command_versions_and_candidate_row_matched".to_string(),
    })
}

fn node_rows(value: &Value) -> Vec<&Value> {
    value
        .get("nodes")
        .and_then(Value::as_array)
        .map(|rows| rows.iter().collect())
        .unwrap_or_default()
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn valid_digest(value: &str) -> Option<&str> {
    (value.len() == 71
        && value.starts_with("sha256:")
        && value[7..].bytes().all(|byte| byte.is_ascii_hexdigit()))
    .then_some(value)
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis())
        .unwrap_or(u64::MAX)
        .max(1)
}

#[cfg(test)]
mod tests;
