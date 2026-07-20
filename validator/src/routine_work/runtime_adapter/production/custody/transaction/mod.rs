use super::super::launch_custody::{
    LaunchBinding, fail_staged, launch_root, observe_staged_cleanup, stage_program,
};
use super::super::production_mediation::{
    allowed_output_scopes, authority_binding, error, owner_process_identity, random_session_id,
    reservation_grant_id, validate_observed_mediation,
};
use super::super::reservation_failure::{
    CapturedCleanup, FailureBinding, PrimaryFailure, TransactionFailure, failure_record,
    finish_error, finish_execution, finish_panic, observe, observed_transition,
};
#[path = "owner.rs"]
mod owner;

use super::observations::*;
use super::store::*;
use super::*;
pub(in crate::routine_work::runtime_adapter::production::custody) use owner::ReservationSpec;
use owner::{ChildHandle, ReservationOwner};
use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};
pub(in crate::routine_work::runtime_adapter::production) use store::{
    AuthorityBinding, OutputComponentJournal, OutputDirectoryIdentity, OutputProvisionJournal,
    OutputStageAmbiguity,
};

pub(in crate::routine_work::runtime_adapter::production) fn mediate_reserved_effect(
    custody: RoutineCustodyCapability,
    context: &LiveContext,
    plan: &RoutinePlan,
    request: RoutineEffectRequest,
    cancellation: RoutineCancellation,
    reuse: PreflightedProductionReuse,
    control: super::super::PublicRoutineControl,
    mut on_reserved: Option<
        &mut dyn FnMut(&super::super::RoutineReservationPublication) -> Result<(), RoutineError>,
    >,
) -> Result<RoutineMediationResult, RoutineError> {
    preflight_production_request(context, plan, &request)?;
    let launch_root = launch_root(&custody)?;
    if !reuse.is_empty() {
        return Err(error("routine-production-session-continuity-required"));
    }
    let binding = authority_binding(&request)?;
    let scopes = allowed_output_scopes(&request);
    let journal = super::super::output_journal::observe(context.worktree_root(), &scopes)?;
    let owner = ReservationOwner::reserve(&custody, &request, binding, journal)?;
    if let Some(publish) = on_reserved.as_mut() {
        let publication = super::super::RoutineReservationPublication::new(
            owner.continuation_id()?,
            owner.recovery_marker(),
            owner.attempt_grant(),
            owner.authenticated_head(),
        );
        publish(&publication)?;
    }
    if control == super::super::PublicRoutineControl::InterruptAfterReservation {
        return interrupted_after_reservation(&request, &owner);
    }
    let output = RefCell::new(None);
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        owner.validate_reserved()?;
        *output.borrow_mut() = Some(super::super::output_journal::begin(
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
                super::super::output_journal::OutputStep::Record(transition) => {
                    owner.record_output(&transition)?;
                    output
                        .borrow_mut()
                        .as_mut()
                        .ok_or_else(|| error("routine-production-output-custody-missing"))?
                        .recorded(transition)?;
                }
                super::super::output_journal::OutputStep::Complete(outcome) => {
                    if outcome.into_ambiguity().is_some() {
                        owner.note_output_ambiguity();
                        return Err(error("routine-production-output-custody-ambiguous"));
                    }
                    break;
                }
            }
        }
        let mut mediation = super::super::super::mediator::begin_effect_mediation(
            context,
            plan,
            request,
            cancellation,
        )?;
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
            DurableSettlement::Complete => {
                super::super::super::mediator::RoutineTerminalOutcome::Complete
            }
            DurableSettlement::Failed => {
                super::super::super::mediator::RoutineTerminalOutcome::Failed
            }
            DurableSettlement::Cancelled => {
                super::super::super::mediator::RoutineTerminalOutcome::Cancelled
            }
            DurableSettlement::Incomplete => {
                super::super::super::mediator::RoutineTerminalOutcome::Incomplete
            }
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
) -> Result<super::super::super::mediator::RoutineContinuationOutcome, RoutineError> {
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
        status: super::super::super::mediator::RoutineMediatorStatus::IncompleteExecution,
        nodes: Vec::new(),
        recovery_marker: Some(owner.recovery_marker()),
        continuation: Some(owner.continuation_id()?),
        attempt_grant: Some(owner.attempt_grant()),
        checkpoint_head: Some(owner.authenticated_head()),
        terminal_outcome: None,
        support_limit: super::super::super::mediator::PRODUCTION_SUPPORT_LIMIT,
    })
}

fn execute_intent(
    owner: &ReservationOwner,
    launch_root: &Path,
    intent: &super::super::super::mediator::IntentExecutionRequest,
) -> Result<super::super::super::mediator::ProcessObservation, RoutineError> {
    let staged = stage_program(
        launch_root,
        LaunchBinding {
            grant_id: owner.failure_binding().grant_id,
            recovery_marker: owner.failure_binding().recovery_marker,
        },
        intent.program(),
    )?;
    let stage = match owner.record_stage(&staged, intent) {
        Ok(stage) => RefCell::new(Some(stage)),
        Err(error) => return Err(fail_staged(&staged, error)),
    };
    let child = RefCell::<Option<ChildHandle>>::new(None);
    let observation = catch_unwind(AssertUnwindSafe(|| {
        match super::super::super::mediator::prepare_authorized_process(
            super::super::super::mediator::AuthorizedProcessPreparation {
                program: &staged.executable,
                root: intent.root(),
                outputs: intent.outputs(),
                reads: intent.reads(),
                argv: intent.argv(),
                environment: intent.environment(),
                framed_input: intent.framed_input().to_vec(),
                output_budget: intent.output_budget(),
                cancellation: intent.cancellation(),
            },
        )? {
            super::super::super::mediator::PreparedProcess::Cancelled(observation) => {
                Ok(observation)
            }
            super::super::super::mediator::PreparedProcess::Suspended(process) => {
                let started = catch_unwind(AssertUnwindSafe(|| {
                    owner.record_started(process.identity()?, &staged.executable, intent)
                }));
                let handle = match started {
                    Ok(Ok(handle)) => handle,
                    Ok(Err(error)) => return Err(process.fail(error)),
                    Err(payload) => process.resume_after_cleanup(payload),
                };
                *child.borrow_mut() = Some(handle);
                process.observe(
                    &staged.executable,
                    intent.root(),
                    intent.outputs(),
                    intent.timeout(),
                    intent.cancellation(),
                )
            }
        }
    }));
    let primary = observe_reaped(owner, observation, &child);
    let mut cleanup = observe_staged_cleanup(&staged);
    if cleanup.evidence() == &CleanupEvidence::Succeeded {
        let recorded = CapturedCleanup::from(catch_unwind(AssertUnwindSafe(|| {
            let handle = stage
                .borrow_mut()
                .take()
                .ok_or_else(|| error("routine-production-launch-custody-missing"))?;
            owner.record_stage_cleaned(handle)
        })));
        cleanup = cleanup.with_transition(recorded.outcome);
    }
    finish_execution(primary, cleanup)
}

fn observe_reaped(
    owner: &ReservationOwner,
    observation: std::thread::Result<
        Result<super::super::super::mediator::ProcessObservation, RoutineError>,
    >,
    child: &RefCell<Option<ChildHandle>>,
) -> std::thread::Result<Result<super::super::super::mediator::ProcessObservation, RoutineError>> {
    match observation {
        Ok(Ok(observation)) => catch_unwind(AssertUnwindSafe(|| {
            let cancelled = observation.termination
                == super::super::super::mediator::ProcessTermination::Cancelled;
            if !cancelled || child.borrow().is_some() {
                let handle = child
                    .borrow_mut()
                    .take()
                    .ok_or_else(|| error("routine-production-child-start-unobserved"))?;
                owner.record_reaped(handle)?;
            }
            Ok(observation)
        })),
        Ok(Err(primary)) => catch_unwind(AssertUnwindSafe(|| {
            if child.borrow().is_some()
                && primary
                    .process_custody()
                    .is_some_and(|value| value.cleanup == CleanupEvidence::Succeeded)
            {
                owner.record_reaped(
                    child
                        .borrow_mut()
                        .take()
                        .ok_or_else(|| error("routine-production-child-start-unobserved"))?,
                )?;
            }
            Err(primary)
        })),
        Err(payload) => Err(payload),
    }
}
