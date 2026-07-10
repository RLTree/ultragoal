use super::super::command_failure::CommandFailureSummary;
use super::super::timing::{NODE_TIMING_REL, VALIDATION_CACHE_REL};
use super::full_command::FullCommandRun;
use super::observation::TelemetryReconciliation;
use super::observation_mode::ObservationMode;
use crate::cli::live_loop::{
    LiveLoopCommand,
    surfaces::{LoopValidationSurface, input_spec_for},
};
use serde_json::Value;
use std::path::Path;
use std::time::Instant;

mod acceptance;
mod fields;
mod node_guards;
mod process_receipt;
mod store;
mod telemetry_reuse;

use acceptance::{has_reconciled_duration, has_replayable_proof, matches_observation_mode};
use fields::{elapsed_ms, expected_result_digest, node_rows, text, valid_digest};
pub(crate) use store::ReplayStore;

pub(super) struct CacheReplay {
    pub(super) run: FullCommandRun,
    pub(super) baseline: FullCommandRun,
    pub(super) prior_result_digest: String,
    pub(super) replayed_output_digest: String,
    pub(super) invalidation_proof: String,
    pub(super) telemetry_reconciliation: TelemetryReconciliation,
    pub(super) source_speed_claim_status: Option<String>,
    pub(super) routine_replay_speed_claim_status: Option<String>,
}

#[cfg(test)]
pub(super) fn verified_local_hit(
    root: &Path,
    surface: LoopValidationSurface,
    candidate: &str,
    input_digest: &str,
    command: &LiveLoopCommand,
    cache_key: &str,
    started: Instant,
    observation_mode: ObservationMode,
) -> Option<CacheReplay> {
    let store = ReplayStore::load(root, command);
    verified_local_hit_from_store(
        &store,
        surface,
        candidate,
        input_digest,
        command,
        cache_key,
        started,
        observation_mode,
    )
}

pub(super) fn verified_local_hit_from_store(
    store: &ReplayStore,
    surface: LoopValidationSurface,
    candidate: &str,
    input_digest: &str,
    command: &LiveLoopCommand,
    cache_key: &str,
    started: Instant,
    observation_mode: ObservationMode,
) -> Option<CacheReplay> {
    if command.cache_mode != "verified-local" {
        return None;
    }
    store.values.iter().find_map(|value| {
        node_rows(value).into_iter().find_map(|row| {
            replay_from_row(
                row,
                surface,
                candidate,
                input_digest,
                command,
                cache_key,
                started,
                observation_mode,
                &store.root,
            )
        })
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
    observation_mode: ObservationMode,
    root: &Path,
) -> Option<CacheReplay> {
    if text(row, "node_id")? != surface.id
        || candidate_mismatch(row, surface, candidate)
        || text(row, "tier")? != command.tier
        || text(row, "cache_mode")? != command.cache_mode
        || text(row, "input_digest")? != input_digest
        || text(row, "current_input_digest")? != input_digest
        || text(row, "canonical_full_command")? != surface.canonical_full_command
        || text(row, "cache_key")? != cache_key
        || text(row, "cache_honesty")? != "pass"
        || text(row, "validation_status")? != "pass"
        || text(row, "validation_cache_status")? != "reusable"
        || text(row, "validator_version")? != crate::cli::live_loop::graph::validator_version()
        || text(row, "law_version")? != crate::cli::live_loop::graph::law_version()
        || text(row, "schema_version")? != crate::cli::live_loop::graph::schema_version()
        || text(row, "fixture_version")? != crate::cli::live_loop::graph::fixture_version()
        || text(row, "runtime_execution_model")?
            != crate::cli::live_loop::graph::runtime_execution_model()
    {
        return None;
    }
    if !matches_surface_input_spec(row, surface) {
        return None;
    }
    if !node_guards::matches_command_identity(row, surface)
        || !node_guards::matches_node_specific_result(row, surface)
        || !node_guards::has_non_self_authored_command_authority(row, surface)
    {
        return None;
    }
    let proof_kind = text(row, "proof_kind")?;
    if !has_replayable_proof(row, proof_kind) {
        return None;
    }
    if !matches_observation_mode(row, observation_mode) {
        return None;
    }
    if !has_reconciled_duration(row) {
        return None;
    }
    let exit_code = row
        .get("exit_status")?
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
    let expected_result_digest =
        expected_result_digest(exit_code, launch_error, &replayed_output_digest);
    let prior_result_digest = valid_digest(text(row, "result_digest")?)?;
    if prior_result_digest != expected_result_digest
        || text(row, "verified_local_result_digest")? != expected_result_digest
    {
        return None;
    }
    if !process_receipt::matches(
        root,
        row,
        surface,
        candidate,
        exit_code,
        launch_error,
        stdout_digest,
        stderr_digest,
        &replayed_output_digest,
        prior_result_digest,
    ) {
        return None;
    }
    let cached_telemetry = telemetry_reuse::reconciliation(row)?;
    let baseline_duration_ms = row
        .get("baseline_duration_ms")?
        .as_u64()
        .filter(|value| *value > 0)?;
    let baseline_exit_code = row
        .get("baseline_exit_code")?
        .as_i64()
        .and_then(|value| i32::try_from(value).ok())?;
    let baseline_launch_error = row.get("baseline_launch_error")?.as_bool()?;
    let baseline_stdout_digest = valid_digest(text(row, "baseline_stdout_digest")?)?;
    let baseline_stderr_digest = valid_digest(text(row, "baseline_stderr_digest")?)?;
    Some(CacheReplay {
        run: FullCommandRun {
            exit_code,
            status_success: true,
            launch_error: false,
            duration_ms: elapsed_ms(started),
            stdout_digest: stdout_digest.to_string(),
            stderr_digest: stderr_digest.to_string(),
            executed_test_count: node_guards::executed_test_count(row, surface),
            failure: Default::default(),
        },
        baseline: FullCommandRun {
            exit_code: baseline_exit_code,
            status_success: !baseline_launch_error && baseline_exit_code == 0,
            launch_error: baseline_launch_error,
            duration_ms: baseline_duration_ms,
            stdout_digest: baseline_stdout_digest.to_string(),
            stderr_digest: baseline_stderr_digest.to_string(),
            executed_test_count: None,
            failure: CommandFailureSummary::from_value(row.get("baseline_failure")),
        },
        prior_result_digest: prior_result_digest.to_string(),
        replayed_output_digest,
        invalidation_proof:
            "cache_key_current_input_digest_command_contract_runtime_model_versions_and_environment_matched"
                .to_string(),
        telemetry_reconciliation: cached_telemetry,
        source_speed_claim_status: text(row, "speed_claim_status").map(str::to_string),
        routine_replay_speed_claim_status: text(row, "routine_replay_speed_claim_status")
            .map(str::to_string),
    })
}

fn candidate_mismatch(row: &Value, surface: LoopValidationSurface, candidate: &str) -> bool {
    node_guards::requires_exact_cargo_identity(surface)
        && text(row, "candidate_digest") != Some(candidate)
}

fn matches_surface_input_spec(row: &Value, surface: LoopValidationSurface) -> bool {
    let Some(spec) = input_spec_for(surface.id) else {
        return false;
    };
    text(row, "surface_input_spec_status") == Some("surface_input_spec_bound")
        && text(row, "surface_input_spec_node_id") == Some(spec.node_id)
        && text(row, "surface_input_spec_cache_boundary") == Some(spec.cache_boundary_name())
        && text(row, "validator_authority") == Some(spec.validator_authority)
        && text(row, "environment_class") == Some(spec.environment_class)
        && text(row, "cache_class") == Some(spec.cache_class)
        && text(row, "claim_surface") == Some(spec.claim_surface)
        && text(row, "output_digest_expectation") == Some(spec.output_digest_expectation)
}

#[cfg(test)]
mod tests;
