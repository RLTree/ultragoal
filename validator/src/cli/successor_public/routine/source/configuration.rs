use super::super::local_store::{RoutineTerminalEvent, append_routine_terminal};
use super::*;
use crate::routine_work::{
    DirtySnapshot, RoutineContinuationOutcome, require_runtime_store_ignored,
};
use crate::state::RoutineFindingBinding;
use crate::{inventory::InventoryBuilder, state::derive_adopted};

pub(crate) const SOURCE_CONFIG_KEY: &str = "contract_id";

pub(crate) fn execute(
    root: &Path,
    invocation: &ParsedInvocation,
    home: Option<&Path>,
) -> RuntimeOutcome {
    if let Some(outcome) = behavior_child::execute_if_requested(invocation) {
        return outcome;
    }
    execute_inner(root, invocation, home).unwrap_or_else(outcome::failure)
}

pub(crate) fn execute_inner(
    root: &Path,
    invocation: &ParsedInvocation,
    home: Option<&Path>,
) -> Result<RuntimeOutcome, PublicFailure> {
    let options = source_context::options(invocation)?;
    let control = match options.interruption() {
        Some(source_context::ReservationInterruption::AfterReservation) => {
            PublicRoutineControl::InterruptAfterReservation
        }
        None => PublicRoutineControl::Run,
    };
    let target = source_context::target_root(root, &options)?;
    let discovery = source_context::discovery_context(&target)?;
    let manifest = manifest::load(&discovery, &target).map_err(PublicFailure::Manifest)?;
    let graph = manifest
        .graph()
        .map_err(|_| PublicFailure::Manifest(manifest::ManifestFailure::Invalid))?;
    let context = source_context::execution_context(&target, &manifest)?;
    validate_selected_sources(&context, &manifest)?;
    let snapshot = LocalDirtyTree::capture(&context).map_err(PublicFailure::Routine)?;
    let plan = plan_routine(&context, &graph, &snapshot, PlanRequest::routine())
        .map_err(PublicFailure::Routine)?;
    let source_id = manifest.source_id();

    if plan.checks().is_empty() {
        let prepared = prepare(&context, &manifest, &graph, &snapshot, &plan)?;
        let result = mediate_public_routine_execution_with_control(
            None,
            &context,
            &plan,
            prepared,
            RoutineCancellation::new(),
            RoutineReuseInput::default(),
            control,
            None,
        )
        .map_err(PublicFailure::Routine)?;
        return Ok(outcome::mediation(
            &result,
            outcome::MediationContext {
                context_id: context.context_id(),
                candidate_id: plan.binding().candidate_id(),
                graph_id: graph.graph_id(),
                snapshot_id: snapshot.snapshot_id(),
                plan_id: plan.plan_id(),
                source_id: &source_id,
                fallback_tool_count: plan.affected_set().coverage().fallback_tool_count(),
            },
        ));
    }

    require_runtime_store_ignored(plan.binding()).map_err(PublicFailure::Routine)?;

    let prepared = prepare(&context, &manifest, &graph, &snapshot, &plan)?;
    let home = home.ok_or(PublicFailure::Host(host::HostFailure::Unavailable))?;
    let state = HostState::open_or_bootstrap(home, &target).map_err(PublicFailure::Host)?;
    let checkpoint = state
        .exact_checkpoint(
            &target,
            context.context_id(),
            plan.binding().candidate_id(),
            plan.plan_id(),
            snapshot.snapshot_id(),
            options.continuation(),
        )
        .map_err(PublicFailure::Host)?;
    let finding_binding = checkpoint
        .as_ref()
        .and_then(|checkpoint| checkpoint.finding_binding().cloned())
        .or_else(|| {
            checkpoint
                .is_none()
                .then(|| capture_finding_binding(&context))
                .flatten()
        });
    if let Some(checkpoint) = checkpoint.as_ref() {
        if checkpoint.is_terminal() && !checkpoint.is_complete() {
            join_terminal_event(&target, &context, &state, plan.binding(), checkpoint)?;
            return Err(PublicFailure::ContinuationUnavailable);
        }
        if checkpoint.is_complete() {
            let recovery_prepared = prepare(&context, &manifest, &graph, &snapshot, &plan)?;
            let result = match reconcile_public_routine_reservation(
                state.issue_custody_capability(),
                &context,
                &plan,
                recovery_prepared,
                checkpoint.attempt_grant(),
                checkpoint.ledger_head(),
            )
            .map_err(PublicFailure::Routine)?
            {
                RoutineContinuationOutcome::Complete(result) => result,
                RoutineContinuationOutcome::Reserved { .. } => {
                    return Err(PublicFailure::PersistenceAfterEffect);
                }
            };
            join_terminal_event(&target, &context, &state, plan.binding(), checkpoint)?;
            return Ok(outcome::mediation(
                &result,
                mediation_context(&context, &plan, &graph, &snapshot, &source_id),
            ));
        }
        if options.continuation().is_none() {
            return Err(PublicFailure::ContinuationUnavailable);
        }
        if !checkpoint.is_reserved() && checkpoint.state() != "reconciled" {
            return Err(PublicFailure::Host(host::HostFailure::Invalid));
        }
        if checkpoint.is_reserved() {
            let recovery_prepared = prepare(&context, &manifest, &graph, &snapshot, &plan)?;
            match reconcile_public_routine_reservation(
                state.issue_custody_capability(),
                &context,
                &plan,
                recovery_prepared,
                checkpoint.attempt_grant(),
                checkpoint.ledger_head(),
            )
            .map_err(PublicFailure::Routine)?
            {
                RoutineContinuationOutcome::Complete(result) => {
                    let continuation = result
                        .continuation()
                        .ok_or(PublicFailure::PersistenceAfterEffect)?;
                    let attempt_grant = result
                        .attempt_grant()
                        .ok_or(PublicFailure::PersistenceAfterEffect)?;
                    let head = result
                        .checkpoint_head()
                        .ok_or(PublicFailure::PersistenceAfterEffect)?;
                    state
                        .record_terminal_checkpoint(
                            &target,
                            context.context_id(),
                            plan.binding().candidate_id(),
                            plan.plan_id(),
                            snapshot.snapshot_id(),
                            continuation,
                            attempt_grant,
                            head,
                            finding_binding.as_ref(),
                            result
                                .terminal_outcome()
                                .ok_or(PublicFailure::PersistenceAfterEffect)?,
                        )
                        .map_err(PublicFailure::Host)?;
                    let persisted = state
                        .exact_checkpoint(
                            &target,
                            context.context_id(),
                            plan.binding().candidate_id(),
                            plan.plan_id(),
                            snapshot.snapshot_id(),
                            Some(continuation),
                        )
                        .map_err(PublicFailure::Host)?
                        .ok_or(PublicFailure::PersistenceAfterEffect)?;
                    join_terminal_event(&target, &context, &state, plan.binding(), &persisted)?;
                    return Ok(outcome::mediation(
                        &result,
                        mediation_context(&context, &plan, &graph, &snapshot, &source_id),
                    ));
                }
                RoutineContinuationOutcome::Reserved { authenticated_head } => state
                    .mark_checkpoint_reconciled(&checkpoint, authenticated_head)
                    .map_err(PublicFailure::Host)?,
            }
        }
    } else if options.continuation().is_some() {
        return Err(PublicFailure::Host(host::HostFailure::Invalid));
    }
    let mut publish_reserved =
        |reservation: &crate::routine_work::RoutineReservationPublication| {
            state
                .record_reserved_checkpoint(
                    &target,
                    context.context_id(),
                    plan.binding().candidate_id(),
                    plan.plan_id(),
                    snapshot.snapshot_id(),
                    reservation.continuation(),
                    reservation.recovery_marker(),
                    reservation.attempt_grant(),
                    reservation.authenticated_ledger_head(),
                    finding_binding.as_ref(),
                )
                .map_err(|_| {
                    crate::routine_work::RoutineError::new(
                        crate::routine_work::RoutineErrorId::ObservationFailed,
                        "routine-host-reservation-checkpoint-failed",
                        None,
                    )
                })
        };
    let mediated = mediate_public_routine_execution_with_control(
        Some(state.issue_custody_capability()),
        &context,
        &plan,
        prepared,
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
        control,
        Some(&mut publish_reserved),
    );
    let result = match mediated {
        Ok(result) => result,
        Err(error) => {
            mark_post_effect_ambiguity(&state, &target, &context, &plan, &snapshot);
            return Err(PublicFailure::Routine(error));
        }
    };

    if control == PublicRoutineControl::Run {
        let continuation = result
            .continuation()
            .ok_or(PublicFailure::PersistenceAfterEffect)?;
        let attempt_grant = result
            .attempt_grant()
            .ok_or(PublicFailure::PersistenceAfterEffect)?;
        let head = result
            .checkpoint_head()
            .ok_or(PublicFailure::PersistenceAfterEffect)?;
        let terminal_outcome = result
            .terminal_outcome()
            .ok_or(PublicFailure::PersistenceAfterEffect)?;
        state
            .record_terminal_checkpoint(
                &target,
                context.context_id(),
                plan.binding().candidate_id(),
                plan.plan_id(),
                snapshot.snapshot_id(),
                continuation,
                attempt_grant,
                head,
                finding_binding.as_ref(),
                terminal_outcome,
            )
            .map_err(|_| {
                mark_post_effect_ambiguity(&state, &target, &context, &plan, &snapshot);
                PublicFailure::PersistenceAfterEffect
            })?;
    }

    if state.verify().is_err() {
        return Err(PublicFailure::PersistenceAfterEffect);
    }
    if result.checkpoint_head().is_some() && control == PublicRoutineControl::Run {
        let continuation = result
            .continuation()
            .ok_or(PublicFailure::PersistenceAfterEffect)?;
        let persisted = state
            .exact_checkpoint(
                &target,
                context.context_id(),
                plan.binding().candidate_id(),
                plan.plan_id(),
                snapshot.snapshot_id(),
                Some(continuation),
            )
            .map_err(PublicFailure::Host)?
            .ok_or(PublicFailure::PersistenceAfterEffect)?;
        join_terminal_event(&target, &context, &state, plan.binding(), &persisted)?;
    }
    Ok(outcome::mediation(
        &result,
        mediation_context(&context, &plan, &graph, &snapshot, &source_id),
    ))
}

fn mark_post_effect_ambiguity(
    state: &HostState,
    target: &Path,
    context: &LiveContext,
    plan: &RoutinePlan,
    snapshot: &DirtySnapshot,
) {
    if let Ok(Some(checkpoint)) = state.exact_checkpoint(
        target,
        context.context_id(),
        plan.binding().candidate_id(),
        plan.plan_id(),
        snapshot.snapshot_id(),
        None,
    ) {
        let _ = state.mark_checkpoint_ambiguous(&checkpoint);
    }
}

fn join_terminal_event(
    target: &Path,
    context: &LiveContext,
    state: &HostState,
    binding: &crate::routine_work::RoutineBinding,
    checkpoint: &host::ContinuationCheckpoint,
) -> Result<(), PublicFailure> {
    if !checkpoint.is_terminal() || checkpoint.state() == "terminal-event-joined" {
        return Ok(());
    };
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

fn capture_finding_binding(context: &LiveContext) -> Option<RoutineFindingBinding> {
    let inventory = InventoryBuilder::new(context).build().ok()?;
    let state = derive_adopted(context, &inventory).ok()?;
    RoutineFindingBinding::from_findings(state.findings())
}

fn mediation_context<'a>(
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
