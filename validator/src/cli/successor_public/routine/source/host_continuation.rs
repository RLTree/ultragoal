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
    let (checkpoint, handoff_alias) = match continuation {
        Some(continuation) => match state
            .resolve_checkpoint(checkpoint_binding, continuation)
            .map_err(PublicFailure::Host)?
        {
            Some(resolution) => (Some(resolution.checkpoint), resolution.handoff_alias),
            None => (None, None),
        },
        None => (
            state
                .exact_checkpoint(checkpoint_binding, None)
                .map_err(PublicFailure::Host)?,
            None,
        ),
    };
    if let Some(alias) = handoff_alias.as_ref() {
        state
            .authenticate_checkpoint(target, alias, true)
            .map_err(PublicFailure::Host)?;
    }
    if let Some(checkpoint) = checkpoint.as_ref() {
        state
            .authenticate_checkpoint(target, checkpoint, false)
            .map_err(PublicFailure::Host)?;
    }
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
                    let terminal_outcome = result
                        .terminal_outcome()
                        .ok_or(PublicFailure::PersistenceAfterEffect)?;
                    state
                        .authenticate_public_state(
                            target,
                            context.context_id(),
                            plan.binding().candidate_id(),
                            plan.plan_id(),
                            snapshot.snapshot_id(),
                            continuation,
                            checkpoint.recovery_marker(),
                            checkpoint.predecessor_continuation(),
                            attempt_grant,
                            head,
                            "terminal-event-pending",
                            Some(terminal_outcome.as_str()),
                            false,
                        )
                        .map_err(PublicFailure::Host)?;
                    state
                        .record_terminal_checkpoint(host::TerminalCheckpoint {
                            binding: checkpoint_binding,
                            continuation,
                            attempt_grant,
                            authenticated_ledger_head: head,
                            finding_binding: finding_binding.as_ref(),
                            terminal_outcome,
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
                    .authenticate_public_state(
                        target,
                        context.context_id(),
                        plan.binding().candidate_id(),
                        plan.plan_id(),
                        snapshot.snapshot_id(),
                        checkpoint.continuation(),
                        checkpoint.recovery_marker(),
                        checkpoint.predecessor_continuation(),
                        checkpoint.attempt_grant(),
                        &authenticated_head,
                        "reconciled",
                        None,
                        false,
                    )
                    .and_then(|_| state.mark_checkpoint_reconciled(checkpoint, authenticated_head))
                    .and_then(|_| {
                        state.remove_alias_if_present(checkpoint_binding, handoff_alias.as_ref())
                    })
                    .map_err(PublicFailure::Host)?,
            }
        }
    } else if continuation.is_some() {
        return Err(PublicFailure::Host(host::HostFailure::Invalid));
    }
    if checkpoint
        .as_ref()
        .is_some_and(|checkpoint| checkpoint.state() == "reconciled")
    {
        state
            .remove_alias_if_present(checkpoint_binding, handoff_alias.as_ref())
            .map_err(PublicFailure::Host)?;
    }
    let predecessor_continuation = checkpoint
        .as_ref()
        .filter(|_| continuation.is_some())
        .map(|checkpoint| checkpoint.continuation().to_owned());
    let mut reservation_publication_failed = false;
    let mut publish_reserved =
        |reservation: &crate::routine_work::RoutineReservationPublication| {
            if state
                .authenticate_public_state(
                    target,
                    context.context_id(),
                    plan.binding().candidate_id(),
                    plan.plan_id(),
                    snapshot.snapshot_id(),
                    reservation.continuation(),
                    reservation.recovery_marker(),
                    predecessor_continuation.as_deref(),
                    reservation.attempt_grant(),
                    reservation.authenticated_ledger_head(),
                    "reserved",
                    None,
                    false,
                )
                .is_err()
            {
                reservation_publication_failed = true;
                return Err(crate::routine_work::RoutineError::new(
                    crate::routine_work::RoutineErrorId::ObservationFailed,
                    "routine-host-reservation-checkpoint-failed",
                    None,
                ));
            }
            if let Err(error) = state.record_reserved_checkpoint(host::ReservedCheckpoint {
                binding: checkpoint_binding,
                continuation: reservation.continuation(),
                recovery_marker: reservation.recovery_marker(),
                attempt_grant: reservation.attempt_grant(),
                authenticated_ledger_head: reservation.authenticated_ledger_head(),
                finding_binding: finding_binding.as_ref(),
            }) {
                let persisted = state
                    .exact_checkpoint(checkpoint_binding, Some(reservation.continuation()))
                    .and_then(|checkpoint| checkpoint.ok_or(host::HostFailure::Invalid))
                    .and_then(|checkpoint| {
                        if !checkpoint.is_reserved()
                            || checkpoint.recovery_marker() != reservation.recovery_marker()
                            || checkpoint.attempt_grant() != reservation.attempt_grant()
                            || checkpoint.ledger_head() != reservation.authenticated_ledger_head()
                        {
                            return Err(host::HostFailure::Invalid);
                        }
                        state.authenticate_checkpoint(target, &checkpoint, false)
                    })
                    .is_ok();
                let _ = error;
                reservation_publication_failed = true;
                return Err(crate::routine_work::RoutineError::new(
                    crate::routine_work::RoutineErrorId::ObservationFailed,
                    if persisted {
                        "routine-host-reservation-checkpoint-ambiguous"
                    } else {
                        "routine-host-reservation-checkpoint-failed"
                    },
                    None,
                ));
            }
            Ok(())
        };
    let mediated = mediate_public_routine_execution_with_control(
        context,
        plan,
        prepared,
        ProductionExecutionControl::with_reservation_publication(
            state.issue_custody_capability(),
            control,
            predecessor_continuation.as_deref(),
            &mut publish_reserved,
        ),
    );
    let result = match mediated {
        Ok(result) => result,
        Err(error) => {
            if !reservation_publication_failed {
                terminal_event::mark_post_effect_ambiguity(&state, target, context, plan, snapshot);
            }
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
        let reserved = state
            .exact_checkpoint(checkpoint_binding, Some(continuation))
            .map_err(PublicFailure::Host)?
            .ok_or(PublicFailure::PersistenceAfterEffect)?;
        if !reserved.is_reserved()
            || reserved.terminal_outcome().is_some()
            || reserved.continuation() != continuation
            || reserved.attempt_grant() != attempt_grant
        {
            return Err(PublicFailure::Host(host::HostFailure::Invalid));
        }
        state
            .authenticate_public_state(
                target,
                context.context_id(),
                plan.binding().candidate_id(),
                plan.plan_id(),
                snapshot.snapshot_id(),
                continuation,
                reserved.recovery_marker(),
                reserved.predecessor_continuation(),
                attempt_grant,
                head,
                "terminal-event-pending",
                Some(terminal_outcome.as_str()),
                false,
            )
            .map_err(PublicFailure::Host)?;
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
