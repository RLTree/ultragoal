use super::super::super::super::local_store::{RoutineTerminalEvent, append_routine_terminal};
use super::*;
use crate::routine_work::DirtySnapshot;
use crate::state::RoutineFindingBinding;
use crate::{inventory::InventoryBuilder, state::derive_adopted};

pub(super) fn mark_post_effect_ambiguity(
    state: &HostState,
    target: &Path,
    context: &LiveContext,
    plan: &RoutinePlan,
    snapshot: &DirtySnapshot,
) {
    let binding = host::CheckpointBinding::new(
        target,
        context.context_id(),
        plan.binding().candidate_id(),
        plan.plan_id(),
        snapshot.snapshot_id(),
    );
    if let Ok(Some(checkpoint)) = state.exact_checkpoint(binding, None) {
        let authenticated = state.authenticate_public_state(
            target,
            checkpoint.context_id(),
            checkpoint.candidate_id(),
            checkpoint.plan_id(),
            checkpoint.snapshot_id(),
            checkpoint.continuation(),
            checkpoint.recovery_marker(),
            checkpoint.predecessor_continuations(),
            checkpoint.attempt_grant(),
            checkpoint.ledger_head(),
            "ambiguous",
            Some("ambiguous"),
            false,
        );
        if authenticated.is_ok() {
            let _ = state.mark_checkpoint_ambiguous(&checkpoint);
        }
    }
}

pub(super) fn join(
    target: &Path,
    context: &LiveContext,
    state: &HostState,
    binding: &crate::routine_work::RoutineBinding,
    checkpoint: &host::ContinuationCheckpoint,
) -> Result<(), PublicFailure> {
    if !checkpoint.is_terminal() || checkpoint.state() == "terminal-event-joined" {
        return Ok(());
    };
    state
        .authenticate_checkpoint(target, checkpoint, false)
        .map_err(PublicFailure::Host)?;
    let event = RoutineTerminalEvent {
        event_id: checkpoint.event_id(),
        continuation_id: checkpoint.continuation(),
        terminal_ledger_head: checkpoint.ledger_head(),
        observed_at_unix_ms: checkpoint.event_observed_at_unix_ms(),
        sequence: checkpoint.event_sequence(),
        parent_event_id: checkpoint.event_parent_id(),
        status: checkpoint.event_status(),
        transition: checkpoint.event_transition(),
        terminal_outcome: checkpoint
            .terminal_outcome()
            .ok_or(PublicFailure::PersistenceAfterEffect)?,
        finding_binding: checkpoint.finding_binding(),
    };
    append_routine_terminal(target, context, binding, event)
        .map_err(|_| PublicFailure::PersistenceAfterEffect)?;
    state
        .mark_event_joined(checkpoint)
        .map_err(PublicFailure::Host)
}

pub(super) fn capture_finding_binding(context: &LiveContext) -> Option<RoutineFindingBinding> {
    let inventory = InventoryBuilder::new(context).build().ok()?;
    let state = derive_adopted(context, &inventory).ok()?;
    RoutineFindingBinding::from_findings(state.findings())
}

pub(super) fn mediation_context<'a>(
    context: &'a LiveContext,
    plan: &'a RoutinePlan,
    graph: &'a ImpactGraph,
    snapshot: &'a DirtySnapshot,
    source_id: &'a str,
) -> outcome::MediationContext<'a> {
    outcome::MediationContext {
        context_id: context.context_id(),
        candidate_id: plan.binding().candidate_id(),
        graph_id: graph.graph_id(),
        snapshot_id: snapshot.snapshot_id(),
        plan_id: plan.plan_id(),
        source_id,
        fallback_tool_count: plan.affected_set().coverage().fallback_tool_count(),
    }
}
