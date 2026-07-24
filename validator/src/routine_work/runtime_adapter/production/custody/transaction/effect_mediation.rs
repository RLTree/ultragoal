use super::super::super::launch_custody::launch_root;
use super::super::super::production_mediation::{
    allowed_output_scopes, authority_binding, error, validate_observed_mediation,
};
use super::super::super::reservation_failure::{
    CapturedCleanup, PrimaryFailure, TransactionFailure, failure_record, finish_error,
    finish_panic, observe, reservation_publication_failure_record,
};
use super::owner::ReservationOwner;
use super::process_execution::execute_intent;
use super::*;
use crate::routine_work::runtime_adapter::mediator;
use crate::routine_work::runtime_adapter::production::{
    PublicRoutineControl, ReservedEffectControl, RoutineReservationPublication, output_journal,
};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};

pub(in crate::routine_work::runtime_adapter::production) fn mediate_reserved_effect(
    custody: RoutineCustodyCapability,
    context: &LiveContext,
    plan: &RoutinePlan,
    request: RoutineEffectRequest,
    control: ReservedEffectControl<'_>,
) -> Result<RoutineMediationResult, RoutineError> {
    let ReservedEffectControl {
        cancellation,
        reuse,
        control,
        predecessor_continuations,
        mut on_reserved,
        ..
    } = control;
    preflight_production_request(context, plan, &request)?;
    let launch_root = launch_root(&custody)?;
    if !reuse.is_empty() {
        return Err(error("routine-production-session-continuity-required"));
    }
    let binding = authority_binding(&request)?;
    let scopes = allowed_output_scopes(&request);
    let journal = output_journal::observe(context.worktree_root(), &scopes)?;
    let owner = ReservationOwner::reserve(
        &custody,
        &request,
        binding,
        journal,
        predecessor_continuations,
    )?;
    if let Some(publish) = on_reserved.as_mut() {
        let publication = RoutineReservationPublication::new(
            owner.continuation_id()?,
            owner.recovery_marker(),
            owner.attempt_grant(),
            owner.authenticated_head(),
        );
        if let Err(error) = publish(&publication) {
            let record = reservation_publication_failure_record(owner.failure_binding(), &error);
            return finish_error(error, None, owner.finish_failure(&record));
        }
    }
    if control == PublicRoutineControl::InterruptAfterReservation {
        return interrupted_after_reservation(&request, &owner);
    }
    let output = RefCell::new(None);
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        owner.validate_reserved()?;
        *output.borrow_mut() = Some(output_journal::begin(
            owner.output_journal(),
            context.worktree_root(),
        )?);
        loop {
            let step = output
                .borrow_mut()
                .as_mut()
                .ok_or_else(|| error("routine-production-output-custody-missing"))?
                .next()?;
            match step {
                output_journal::OutputStep::Record(transition) => {
                    owner.record_output(&transition)?;
                    output
                        .borrow_mut()
                        .as_mut()
                        .ok_or_else(|| error("routine-production-output-custody-missing"))?
                        .recorded(transition)?;
                }
                output_journal::OutputStep::Complete(outcome) => {
                    if outcome.into_ambiguity().is_some() {
                        owner.note_output_ambiguity();
                        return Err(error("routine-production-output-custody-ambiguous"));
                    }
                    break;
                }
            }
        }
        let mut mediation = mediator::begin_effect_mediation(context, plan, request, cancellation)?;
        while let Some(intent) = mediation.next_intent()? {
            let observation = execute_intent(&owner, &launch_root, &intent)?;
            mediation.observe_intent(intent, observation)?;
        }
        let observed = mediation.finish()?;
        let (mut result, settlement, artifacts, claimed) = observed.into_parts();
        let authenticated = validate_observed_mediation(&result, settlement, &artifacts, &claimed)?;
        let settlement_artifacts = if settlement == DurableSettlement::Complete {
            authenticated
        } else {
            BTreeMap::new()
        };
        owner.settle(settlement, &result, &settlement_artifacts)?;
        result.continuation = Some(owner.continuation_id()?);
        result.attempt_grant = Some(owner.attempt_grant());
        result.checkpoint_head = Some(owner.authenticated_head());
        result.terminal_outcome = Some(match settlement {
            DurableSettlement::Complete => mediator::RoutineTerminalOutcome::Complete,
            DurableSettlement::Failed => mediator::RoutineTerminalOutcome::Failed,
            DurableSettlement::Cancelled => mediator::RoutineTerminalOutcome::Cancelled,
            DurableSettlement::Incomplete => mediator::RoutineTerminalOutcome::Incomplete,
        });
        output
            .borrow_mut()
            .take()
            .ok_or_else(|| error("routine-production-output-custody-missing"))?
            .accept();
        Ok(result)
    }));
    let primary = match outcome {
        Ok(Ok(result)) => return Ok(result),
        Ok(Err(error)) => PrimaryFailure::Error(error),
        Err(payload) => PrimaryFailure::Panic(payload),
    };
    let cleanup = CapturedCleanup::from(catch_unwind(AssertUnwindSafe(|| {
        output
            .borrow_mut()
            .take()
            .map_or(Ok(()), |value| value.abort())
    })));
    match observe(primary, cleanup) {
        TransactionFailure::Error(error, parts, cleanup) => {
            let record = failure_record(owner.failure_binding(), owner.started(), parts);
            finish_error(error, cleanup, owner.finish_failure(&record))
        }
        TransactionFailure::Panic(payload, parts) => {
            let record = failure_record(owner.failure_binding(), owner.started(), parts);
            finish_panic(payload, owner.finish_failure(&record))
        }
    }
}

pub(in crate::routine_work::runtime_adapter::production) fn reconcile_reserved_effect(
    custody: RoutineCustodyCapability,
    request: &RoutineEffectRequest,
    attempt_grant: &str,
    expected_head: &str,
) -> Result<mediator::RoutineContinuationOutcome, RoutineError> {
    let binding = authority_binding(request)?;
    DurableCustody::reconcile_reserved(&custody, &binding, attempt_grant, expected_head)
}

fn interrupted_after_reservation(
    request: &RoutineEffectRequest,
    owner: &ReservationOwner,
) -> Result<RoutineMediationResult, RoutineError> {
    Ok(RoutineMediationResult {
        request_id: Some(request.request_id().to_owned()),
        protocol_id: Some(request.protocol_id().to_owned()),
        status: mediator::RoutineMediatorStatus::IncompleteExecution,
        nodes: Vec::new(),
        recovery_marker: Some(owner.recovery_marker()),
        continuation: Some(owner.continuation_id()?),
        attempt_grant: Some(owner.attempt_grant()),
        checkpoint_head: Some(owner.authenticated_head()),
        terminal_outcome: None,
        support_limit: mediator::PRODUCTION_SUPPORT_LIMIT,
    })
}
