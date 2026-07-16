use super::super::launch_custody::{
    LaunchBinding, cleanup_staged, fail_staged, launch_root, stage_program,
};
use super::super::production_mediation::{
    allowed_output_scopes, authority_binding, error, owner_process_identity, random_session_id,
    reservation_grant_id, validate_observed_mediation,
};
use super::super::reservation_failure::{
    CapturedCleanup, FailureBinding, PrimaryFailure, TransactionFailure, failure_record,
    finish_error, finish_execution, finish_panic, observe, observed_transition,
};
#[path = "transaction/owner.rs"]
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
    authority_root: &Path,
    context: &LiveContext,
    plan: &RoutinePlan,
    request: RoutineEffectRequest,
    cancellation: RoutineCancellation,
    reuse: PreflightedProductionReuse,
) -> Result<RoutineMediationResult, RoutineError> {
    preflight_production_request(context, plan, &request)?;
    let launch_root = launch_root(authority_root)?;
    if !reuse.is_empty() {
        return Err(error("routine-production-session-continuity-required"));
    }
    let binding = authority_binding(&request)?;
    let scopes = allowed_output_scopes(&request);
    let journal = super::super::output_journal::observe(context.worktree_root(), &scopes)?;
    let owner = ReservationOwner::reserve(authority_root, &request, binding, journal)?;
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
        let (result, settlement, artifacts, claimed) = observed.into_parts();
        let authenticated = validate_observed_mediation(&result, settlement, &artifacts, &claimed)?;
        let settlement_artifacts = if settlement == DurableSettlement::Complete {
            authenticated
        } else {
            BTreeMap::new()
        };
        owner.settle(settlement, &result, &settlement_artifacts)?;
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
            &staged.executable,
            intent.root(),
            intent.outputs(),
            intent.reads(),
            intent.argv(),
            intent.environment(),
            intent.framed_input().to_vec(),
            intent.output_budget(),
            intent.cancellation(),
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
    let mut cleanup =
        CapturedCleanup::from(catch_unwind(AssertUnwindSafe(|| cleanup_staged(&staged))));
    if cleanup.evidence == CleanupEvidence::Succeeded {
        let recorded = CapturedCleanup::from(catch_unwind(AssertUnwindSafe(|| {
            let handle = stage
                .borrow_mut()
                .take()
                .ok_or_else(|| error("routine-production-launch-custody-missing"))?;
            owner.record_stage_cleaned(handle)
        })));
        if recorded.evidence != CleanupEvidence::Succeeded {
            cleanup = recorded;
        }
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
