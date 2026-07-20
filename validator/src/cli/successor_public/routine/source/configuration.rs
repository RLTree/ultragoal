use super::super::local_store::{RoutineTerminalEvent, append_routine_terminal};
use super::*;
use crate::routine_work::{DirtySnapshot, RoutineContinuationOutcome};
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
    if let Some(checkpoint) = checkpoint {
        if checkpoint.is_complete() {
            let recovery_prepared = prepare(&context, &manifest, &graph, &snapshot, &plan)?;
            let result = match reconcile_public_routine_reservation(
                state.authority_root(),
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
            join_terminal_event(&target, &state, &checkpoint);
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
                state.authority_root(),
                &context,
                &plan,
                recovery_prepared,
                checkpoint.attempt_grant(),
                checkpoint.ledger_head(),
            )
            .map_err(PublicFailure::Routine)?
            {
                RoutineContinuationOutcome::Complete(result) => {
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
    let mediated = mediate_public_routine_execution_with_control(
        Some(state.authority_root()),
        &context,
        &plan,
        prepared,
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
        control,
    )
    .map_err(PublicFailure::Routine);
    let result = mediated?;

    if control == PublicRoutineControl::InterruptAfterReservation {
        let continuation = result
            .continuation()
            .ok_or(PublicFailure::PersistenceAfterEffect)?;
        let recovery_marker = result
            .recovery_marker()
            .ok_or(PublicFailure::PersistenceAfterEffect)?;
        let attempt_grant = result
            .attempt_grant()
            .ok_or(PublicFailure::PersistenceAfterEffect)?;
        let head = result
            .checkpoint_head()
            .ok_or(PublicFailure::PersistenceAfterEffect)?;
        state
            .record_reserved_checkpoint(
                &target,
                context.context_id(),
                plan.binding().candidate_id(),
                plan.plan_id(),
                snapshot.snapshot_id(),
                continuation,
                recovery_marker,
                attempt_grant,
                head,
            )
            .map_err(PublicFailure::Host)?;
    } else if result.checkpoint_head().is_some() {
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
            .record_complete_checkpoint(
                &target,
                context.context_id(),
                plan.binding().candidate_id(),
                plan.plan_id(),
                snapshot.snapshot_id(),
                continuation,
                attempt_grant,
                head,
            )
            .map_err(PublicFailure::Host)?;
    }

    if state.verify().is_err() {
        return Err(PublicFailure::PersistenceAfterEffect);
    }
    Ok(outcome::mediation(
        &result,
        mediation_context(&context, &plan, &graph, &snapshot, &source_id),
    ))
}

fn join_terminal_event(
    target: &Path,
    state: &HostState,
    checkpoint: &host::ContinuationCheckpoint,
) {
    if !checkpoint.is_complete() || checkpoint.state() == "complete-event-joined" {
        return;
    }
    let Ok(context) = super::super::read_context(target) else {
        return;
    };
    let Ok(inventory) = InventoryBuilder::new(&context).build() else {
        return;
    };
    let Ok(product_state) = derive_adopted(&context, &inventory) else {
        return;
    };
    let findings = product_state
        .findings()
        .first()
        .map(std::slice::from_ref)
        .unwrap_or(&[]);
    let event = RoutineTerminalEvent {
        event_id: checkpoint.event_id(),
        continuation_id: checkpoint.continuation(),
        terminal_ledger_head: checkpoint.ledger_head(),
        observed_at_unix_ms: checkpoint.event_observed_at_unix_ms(),
        sequence: checkpoint.event_sequence(),
        parent_event_id: checkpoint.event_parent_id(),
        status: checkpoint.event_status(),
        transition: checkpoint.event_transition(),
        findings,
    };
    if append_routine_terminal(target, &context, event).is_ok() {
        let _ = state.mark_event_joined(checkpoint);
    }
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
