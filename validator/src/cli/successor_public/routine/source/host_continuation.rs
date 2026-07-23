use super::*;
use crate::cli::successor_public::local_store::RUNTIME_SOURCE_ID;
use crate::routine_work::{
    DirtySnapshot, RoutineContinuationOutcome, require_runtime_store_ignored,
};

pub(super) struct HostContinuationRequest<'a> {
    pub(super) target: &'a Path,
    pub(super) context: &'a LiveContext,
    pub(super) manifest: &'a LoadedManifest,
    pub(super) graph: &'a ImpactGraph,
    pub(super) snapshot: &'a DirtySnapshot,
    pub(super) plan: &'a RoutinePlan,
    pub(super) source_id: &'a str,
    pub(super) continuation: Option<&'a str>,
    pub(super) control: PublicRoutineControl,
    pub(super) home: Option<&'a Path>,
}

pub(super) fn run(request: HostContinuationRequest<'_>) -> Result<RuntimeOutcome, PublicFailure> {
    let HostContinuationRequest {
        target,
        context,
        manifest,
        graph,
        snapshot,
        plan,
        source_id,
        continuation,
        control,
        home,
    } = request;
    require_runtime_store_ignored(plan.binding(), RUNTIME_SOURCE_ID)
        .map_err(PublicFailure::Routine)?;

    let prepared = prepare(context, manifest, graph, snapshot, plan)?;
    let home = home.ok_or(PublicFailure::Host(host::HostFailure::Unavailable))?;
    let state = HostState::open_or_bootstrap(home, target).map_err(PublicFailure::Host)?;
    let checkpoint_binding = host::CheckpointBinding::new(
        target,
        context.context_id(),
        plan.binding().candidate_id(),
        plan.plan_id(),
        snapshot.snapshot_id(),
    );
    let checkpoint = state
        .exact_checkpoint(checkpoint_binding, continuation)
        .map_err(PublicFailure::Host)?;
    let finding_binding = checkpoint
        .as_ref()
        .and_then(|checkpoint| checkpoint.finding_binding().cloned())
        .or_else(|| {
            checkpoint
                .is_none()
                .then(|| terminal_event::capture_finding_binding(context))
                .flatten()
        });
    if let Some(checkpoint) = checkpoint.as_ref() {
        if checkpoint.is_terminal() && !checkpoint.is_complete() {
            terminal_event::join(target, context, &state, plan.binding(), checkpoint)?;
            return Err(PublicFailure::ContinuationUnavailable);
        }
        if checkpoint.is_complete() {
            let recovery_prepared = prepare(context, manifest, graph, snapshot, plan)?;
            let result = match reconcile_public_routine_reservation(
                state.issue_custody_capability(),
                context,
                plan,
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
            terminal_event::join(target, context, &state, plan.binding(), checkpoint)?;
            return Ok(outcome::mediation(
                &result,
                terminal_event::mediation_context(context, plan, graph, snapshot, source_id),
            ));
        }
        if continuation.is_none() {
            return Err(PublicFailure::ContinuationUnavailable);
        }
        if !checkpoint.is_reserved() && checkpoint.state() != "reconciled" {
            return Err(PublicFailure::Host(host::HostFailure::Invalid));
        }
        if checkpoint.is_reserved() {
            let recovery_prepared = prepare(context, manifest, graph, snapshot, plan)?;
            match reconcile_public_routine_reservation(
                state.issue_custody_capability(),
                context,
                plan,
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
                        .record_terminal_checkpoint(host::TerminalCheckpoint {
                            binding: checkpoint_binding,
                            continuation,
                            attempt_grant,
                            authenticated_ledger_head: head,
                            finding_binding: finding_binding.as_ref(),
                            terminal_outcome: result
                                .terminal_outcome()
                                .ok_or(PublicFailure::PersistenceAfterEffect)?,
                        })
                        .map_err(PublicFailure::Host)?;
                    let persisted = state
                        .exact_checkpoint(checkpoint_binding, Some(continuation))
                        .map_err(PublicFailure::Host)?
                        .ok_or(PublicFailure::PersistenceAfterEffect)?;
                    terminal_event::join(target, context, &state, plan.binding(), &persisted)?;
                    return Ok(outcome::mediation(
                        &result,
                        terminal_event::mediation_context(
                            context, plan, graph, snapshot, source_id,
                        ),
                    ));
                }
                RoutineContinuationOutcome::Reserved { authenticated_head } => state
                    .mark_checkpoint_reconciled(checkpoint, authenticated_head)
                    .map_err(PublicFailure::Host)?,
            }
        }
    } else if continuation.is_some() {
        return Err(PublicFailure::Host(host::HostFailure::Invalid));
    }
    let mut publish_reserved =
        |reservation: &crate::routine_work::RoutineReservationPublication| {
            state
                .record_reserved_checkpoint(host::ReservedCheckpoint {
                    binding: checkpoint_binding,
                    continuation: reservation.continuation(),
                    recovery_marker: reservation.recovery_marker(),
                    attempt_grant: reservation.attempt_grant(),
                    authenticated_ledger_head: reservation.authenticated_ledger_head(),
                    finding_binding: finding_binding.as_ref(),
                })
                .map_err(|_| {
                    crate::routine_work::RoutineError::new(
                        crate::routine_work::RoutineErrorId::ObservationFailed,
                        "routine-host-reservation-checkpoint-failed",
                        None,
                    )
                })
        };
    let mediated = mediate_public_routine_execution_with_control(
        context,
        plan,
        prepared,
        ProductionExecutionControl::with_reservation_publication(
            state.issue_custody_capability(),
            control,
            &mut publish_reserved,
        ),
    );
    let result = match mediated {
        Ok(result) => result,
        Err(error) => {
            terminal_event::mark_post_effect_ambiguity(&state, target, context, plan, snapshot);
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
            .record_terminal_checkpoint(host::TerminalCheckpoint {
                binding: checkpoint_binding,
                continuation,
                attempt_grant,
                authenticated_ledger_head: head,
                finding_binding: finding_binding.as_ref(),
                terminal_outcome,
            })
            .map_err(|_| {
                terminal_event::mark_post_effect_ambiguity(&state, target, context, plan, snapshot);
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
            .exact_checkpoint(checkpoint_binding, Some(continuation))
            .map_err(PublicFailure::Host)?
            .ok_or(PublicFailure::PersistenceAfterEffect)?;
        terminal_event::join(target, context, &state, plan.binding(), &persisted)?;
    }
    Ok(outcome::mediation(
        &result,
        terminal_event::mediation_context(context, plan, graph, snapshot, source_id),
    ))
}
