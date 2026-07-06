use super::verified_work::VerifiedLocalProof;

pub(super) fn output_digest(verified_local: &VerifiedLocalProof) -> String {
    crate::digest::bytes(
        format!(
            "stdout={};stderr={}",
            verified_local.actual_work.stdout_digest, verified_local.actual_work.stderr_digest
        )
        .as_bytes(),
    )
}

pub(super) fn result_digest(verified_local: &VerifiedLocalProof, output_digest: &str) -> String {
    crate::digest::bytes(
        format!(
            "exit={};launch={};output={}",
            verified_local.actual_work.exit_code,
            verified_local.actual_work.launch_error,
            output_digest
        )
        .as_bytes(),
    )
}

pub(super) fn proof_surface(verified_local: &VerifiedLocalProof) -> &'static str {
    match verified_local.proof_kind {
        "executed" => {
            "executed current-candidate command with exit status, work units, digests, and timing receipt"
        }
        "verified_cache_hit" => {
            "verified same-candidate cache replay with current input digests and equivalence proof"
        }
        _ => "invalid proof_kind; row is blocked",
    }
}

pub(super) fn reconciled_command_duration_ms(verified_local: &VerifiedLocalProof) -> u64 {
    verified_local
        .actual_work
        .duration_ms
        .saturating_add(verified_local.graph_overhead_ms)
        .saturating_add(verified_local.telemetry_reconciliation_duration_ms)
}

pub(super) fn baseline_reuse_fields(proof_kind: &str) -> (&'static str, &'static str) {
    match proof_kind {
        "verified_cache_hit" => (
            "verified_baseline_reuse",
            "baseline_reused_from_same_candidate_current_input_timing_row",
        ),
        _ => (
            "executed",
            "baseline_command_executed_for_current_measurement",
        ),
    }
}
