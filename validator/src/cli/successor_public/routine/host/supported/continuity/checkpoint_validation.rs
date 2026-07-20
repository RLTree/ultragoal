use super::super::HostFailure;
use super::checkpoint::{ContinuationCheckpoint, RoutineCheckpointOperation};
use sha2::{Digest, Sha256};
use std::path::Path;

pub(super) fn checkpoint(
    target: &Path,
    context_id: &str,
    candidate_id: &str,
    plan_id: &str,
    snapshot_id: &str,
    continuation: &str,
    recovery_marker: &str,
    attempt_grant: &str,
    authenticated_ledger_head: &str,
    finding_binding: Option<&crate::state::RoutineFindingBinding>,
    terminal_outcome: Option<crate::routine_work::RoutineTerminalOutcome>,
    state: &str,
    generation: u64,
) -> Result<ContinuationCheckpoint, HostFailure> {
    if !target.is_absolute()
        || target.to_str().is_none()
        || !continuation.starts_with("routine-cont-")
        || attempt_grant.is_empty()
        || authenticated_ledger_head.is_empty()
        || generation == 0
    {
        return Err(HostFailure::Invalid);
    }
    Ok(ContinuationCheckpoint {
        schema_version: "RoutineContinuationCheckpoint-v4".to_owned(),
        generation,
        target: target.to_str().ok_or(HostFailure::Invalid)?.to_owned(),
        context_id: context_id.to_owned(),
        candidate_id: candidate_id.to_owned(),
        plan_id: plan_id.to_owned(),
        snapshot_id: snapshot_id.to_owned(),
        continuation: continuation.to_owned(),
        recovery_marker: recovery_marker.to_owned(),
        attempt_grant: attempt_grant.to_owned(),
        authenticated_ledger_head: authenticated_ledger_head.to_owned(),
        finding_binding: finding_binding.cloned(),
        operation: RoutineCheckpointOperation::Terminal,
        terminal_outcome,
        state: state.to_owned(),
        event_id: terminal_event_id(continuation, authenticated_ledger_head),
        event_observed_at_unix_ms: 0,
        event_sequence: generation,
        event_parent_id: None,
        event_status: event_projection(terminal_outcome).0.to_owned(),
        event_transition: event_projection(terminal_outcome).1.to_owned(),
    })
}

pub(super) fn validate_checkpoint(
    checkpoint: &ContinuationCheckpoint,
    target: &Path,
    context_id: &str,
    candidate_id: &str,
    plan_id: &str,
    snapshot_id: &str,
    continuation: Option<&str>,
) -> Result<(), HostFailure> {
    if checkpoint.schema_version != "RoutineContinuationCheckpoint-v4"
        || checkpoint.generation == 0
        || !matches!(
            checkpoint.state.as_str(),
            "reserved"
                | "reconciled"
                | "ambiguous"
                | "terminal-event-pending"
                | "terminal-event-joined"
        )
        || checkpoint.target != target.to_str().ok_or(HostFailure::Invalid)?
        || checkpoint.context_id != context_id
        || checkpoint.candidate_id != candidate_id
        || checkpoint.plan_id != plan_id
        || checkpoint.snapshot_id != snapshot_id
        || !checkpoint.continuation.starts_with("routine-cont-")
        || checkpoint.attempt_grant.is_empty()
        || checkpoint.authenticated_ledger_head.is_empty()
        || checkpoint.operation != RoutineCheckpointOperation::Terminal
        || checkpoint.event_id
            != terminal_event_id(
                &checkpoint.continuation,
                &checkpoint.authenticated_ledger_head,
            )
        || checkpoint.event_sequence == 0
        || checkpoint.event_status.is_empty()
        || checkpoint.event_transition.is_empty()
        || (
            checkpoint.event_status.as_str(),
            checkpoint.event_transition.as_str(),
        ) != event_projection(checkpoint.terminal_outcome)
        || checkpoint
            .finding_binding
            .as_ref()
            .is_some_and(|binding| binding.finding_id.is_empty() || binding.repair_id.is_empty())
        || (checkpoint.state == "reserved" && checkpoint.recovery_marker.is_empty())
        || ((checkpoint.is_reserved() || checkpoint.state == "reconciled")
            && checkpoint.terminal_outcome.is_some())
        || (checkpoint.is_terminal()
            && checkpoint.terminal_outcome.is_none_or(|outcome| {
                outcome == crate::routine_work::RoutineTerminalOutcome::Ambiguous
            }))
        || (checkpoint.state == "ambiguous"
            && checkpoint.terminal_outcome
                != Some(crate::routine_work::RoutineTerminalOutcome::Ambiguous))
        || continuation.is_some_and(|value| checkpoint.continuation != value)
    {
        return Err(HostFailure::Invalid);
    }
    Ok(())
}

pub(super) fn event_projection(
    terminal_outcome: Option<crate::routine_work::RoutineTerminalOutcome>,
) -> (&'static str, &'static str) {
    match terminal_outcome {
        Some(crate::routine_work::RoutineTerminalOutcome::Complete) => ("pass", "executed"),
        Some(crate::routine_work::RoutineTerminalOutcome::Failed) => ("fail", "failed"),
        Some(crate::routine_work::RoutineTerminalOutcome::Cancelled) => ("blocked", "cancelled"),
        Some(crate::routine_work::RoutineTerminalOutcome::Incomplete)
        | Some(crate::routine_work::RoutineTerminalOutcome::Ambiguous)
        | None => ("blocked", "interrupted"),
    }
}

pub(super) fn terminal_event_id(continuation: &str, terminal_head: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(b"routine-terminal-event-v1\0");
    digest.update(continuation.as_bytes());
    digest.update(b"\0");
    digest.update(terminal_head.as_bytes());
    format!("routine-terminal-{:x}", digest.finalize())
}
