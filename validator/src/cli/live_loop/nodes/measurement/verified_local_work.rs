use super::super::super::graph;
use super::cache_replay;
use super::full_command::{FullCommandRun, run_narrow_command};
use super::observation;
use super::observation_mode::ObservationMode;
use super::timing::verified_work::VerifiedLocalProof;
use crate::cli::live_loop::{LiveLoopCommand, surfaces::LoopValidationSurface};
use std::path::Path;
use std::time::Instant;

pub(super) fn cached_verified_local(
    root: &Path,
    surface: LoopValidationSurface,
    candidate: &str,
    input_digest: &str,
    command: &LiveLoopCommand,
    observation_mode: ObservationMode,
) -> Option<(FullCommandRun, VerifiedLocalProof)> {
    let started = Instant::now();
    let cache_key = graph::verified_local_cache_key(
        surface.id,
        input_digest,
        &command.tier,
        &command.cache_mode,
    );
    let graph_overhead_ms = elapsed_ms(started);
    let replay_started = Instant::now();
    let mut cached = cache_replay::verified_local_hit(
        root,
        surface,
        candidate,
        input_digest,
        command,
        &cache_key,
        replay_started,
        observation_mode,
    )?;
    let telemetry_reconciliation = cached.telemetry_reconciliation.clone();
    cached.run.failure = telemetry_reconciliation.failure_summary();
    let proof = VerifiedLocalProof {
        proof_kind: "verified_cache_hit",
        cache_hit: true,
        cache_key,
        graph_overhead_ms,
        actual_work: cached.run,
        work_unit_count: 0,
        equivalence_status: "verified_same_candidate_cache_replay".to_string(),
        invalidation_proof: cached.invalidation_proof,
        telemetry_reconciliation_status: telemetry_reconciliation.status.clone(),
        telemetry_reconciliation_duration_ms: telemetry_reconciliation.duration_ms,
        telemetry_reconciliation,
        prior_result_digest: Some(cached.prior_result_digest),
        replayed_output_digest: Some(cached.replayed_output_digest),
        cache_equivalence_status: Some("pass".to_string()),
    };
    Some((cached.baseline, proof))
}

pub(super) fn executed_verified_local(
    root: &Path,
    surface: LoopValidationSurface,
    candidate: &str,
    input_digest: &str,
    command: &LiveLoopCommand,
    observation_mode: ObservationMode,
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
    let telemetry_reconciliation = match observation_mode {
        ObservationMode::FullRoundtrip => {
            observation::reconcile(root, surface, candidate, command, &actual_work)
        }
        ObservationMode::LoopRunSnapshot => {
            observation::loop_run_snapshot_pending(surface, candidate, command, &actual_work)
        }
    };
    let telemetry_failure = telemetry_reconciliation.failure_summary();
    if telemetry_failure.has_details() {
        actual_work.failure = telemetry_failure;
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

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis())
        .unwrap_or(u64::MAX)
        .max(1)
}
