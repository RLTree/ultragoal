use super::super::super::super::local_store::append_routine_terminal;
use super::*;
use crate::routine_work::DirtySnapshot;
use crate::state::RoutineFindingBinding;

pub(super) fn mark_post_effect_ambiguity(
    state: &HostState,
    target: &Path,
    context: &LiveContext,
    plan: &RoutinePlan,
    snapshot: &DirtySnapshot,
    execution_id: &str,
) {
    let binding = host::CheckpointBinding::new(
        target,
        context.context_id(),
        plan.binding().candidate_id(),
        plan.plan_id(),
        snapshot.snapshot_id(),
        execution_id,
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
    if !checkpoint.is_terminal() {
        return Ok(());
    };
    if checkpoint.state() == "terminal-event-joined" {
        return state
            .verify_joined_terminal_event(target, context, checkpoint)
            .map_err(PublicFailure::Host);
    }
    state
        .authenticate_checkpoint(target, checkpoint, false)
        .map_err(PublicFailure::Host)?;
    let _ = binding;
    append_routine_terminal(state, target, context, checkpoint)
        .map_err(|_| PublicFailure::PersistenceAfterEffect)?;
    state
        .mark_event_joined(target, context, checkpoint)
        .map_err(PublicFailure::Host)
}

pub(super) fn capture_finding_binding(_context: &LiveContext) -> Option<RoutineFindingBinding> {
    // A routine context is the representative target repository, not the
    // UltraGoal source root. Do not substitute a target inventory for current
    // product authority; the optional pre-effect product binding is withheld.
    None
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
