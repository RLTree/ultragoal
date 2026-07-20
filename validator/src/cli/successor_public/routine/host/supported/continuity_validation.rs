use super::super::super::{CheckpointBinding, HostFailure};
use super::checkpoint::{ContinuationCheckpoint, RoutineCheckpointOperation};
use sha2::{Digest, Sha256};

pub(super) struct CheckpointDraft<'a> {
    pub(super) binding: CheckpointBinding<'a>,
    pub(super) continuation: &'a str,
    pub(super) recovery_marker: &'a str,
    pub(super) attempt_grant: &'a str,
    pub(super) authenticated_ledger_head: &'a str,
    pub(super) finding_binding: Option<&'a crate::state::RoutineFindingBinding>,
    pub(super) terminal_outcome: Option<crate::routine_work::RoutineTerminalOutcome>,
    pub(super) state: &'a str,
    pub(super) generation: u64,
}

pub(super) fn checkpoint(
    draft: CheckpointDraft<'_>,
) -> Result<ContinuationCheckpoint, HostFailure> {
    if !draft.binding.target().is_absolute()
        || draft.binding.target().to_str().is_none()
        || !draft.continuation.starts_with("routine-cont-")
        || draft.attempt_grant.is_empty()
        || draft.authenticated_ledger_head.is_empty()
        || draft.generation == 0
    {
        return Err(HostFailure::Invalid);
    }
    Ok(ContinuationCheckpoint {
        schema_version: "RoutineContinuationCheckpoint-v4".to_owned(),
        generation: draft.generation,
        target: draft
            .binding
            .target()
            .to_str()
            .ok_or(HostFailure::Invalid)?
            .to_owned(),
        context_id: draft.binding.context_id().to_owned(),
        candidate_id: draft.binding.candidate_id().to_owned(),
        plan_id: draft.binding.plan_id().to_owned(),
        snapshot_id: draft.binding.snapshot_id().to_owned(),
        continuation: draft.continuation.to_owned(),
        recovery_marker: draft.recovery_marker.to_owned(),
        attempt_grant: draft.attempt_grant.to_owned(),
        authenticated_ledger_head: draft.authenticated_ledger_head.to_owned(),
        finding_binding: draft.finding_binding.cloned(),
        operation: RoutineCheckpointOperation::Terminal,
        terminal_outcome: draft.terminal_outcome,
        state: draft.state.to_owned(),
        event_id: terminal_event_id(draft.continuation, draft.authenticated_ledger_head),
        event_observed_at_unix_ms: 0,
        event_sequence: draft.generation,
        event_parent_id: None,
        event_status: event_projection(draft.terminal_outcome).0.to_owned(),
        event_transition: event_projection(draft.terminal_outcome).1.to_owned(),
    })
}

pub(super) fn validate_checkpoint(
    checkpoint: &ContinuationCheckpoint,
    binding: CheckpointBinding<'_>,
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
        || checkpoint.target != binding.target().to_str().ok_or(HostFailure::Invalid)?
        || checkpoint.context_id != binding.context_id()
        || checkpoint.candidate_id != binding.candidate_id()
        || checkpoint.plan_id != binding.plan_id()
        || checkpoint.snapshot_id != binding.snapshot_id()
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
