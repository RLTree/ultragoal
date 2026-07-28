use super::super::super::launch_custody::{
    LaunchBinding, fail_staged, observe_staged_cleanup, stage_program,
};
use super::super::super::production_mediation::error;
use super::super::super::reservation_failure::{CapturedCleanup, finish_execution};
use super::owner::{ChildHandle, ReservationOwner};
use super::*;
use crate::routine_work::runtime_adapter::mediator;
use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};

pub(super) fn execute_intent(
    owner: &ReservationOwner,
    launch_root: &Path,
    intent: &mediator::IntentExecutionRequest,
) -> Result<mediator::ProcessObservation, RoutineError> {
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
    let observation = catch_unwind(AssertUnwindSafe(
        || match mediator::prepare_authorized_process(mediator::AuthorizedProcessPreparation {
            program: &staged.executable,
            root: intent.root(),
            outputs: intent.outputs(),
            reads: intent.reads(),
            argv: intent.argv(),
            environment: intent.environment(),
            framed_input: intent.framed_input().to_vec(),
            output_budget: intent.output_budget(),
            cancellation: intent.cancellation(),
        })? {
            mediator::PreparedProcess::Cancelled(observation) => Ok(observation),
            mediator::PreparedProcess::Suspended(process) => {
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
        },
    ));
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
    observation: std::thread::Result<Result<mediator::ProcessObservation, RoutineError>>,
    child: &RefCell<Option<ChildHandle>>,
) -> std::thread::Result<Result<mediator::ProcessObservation, RoutineError>> {
    match observation {
        Ok(Ok(observation)) => catch_unwind(AssertUnwindSafe(|| {
            let cancelled = observation.termination == mediator::ProcessTermination::Cancelled;
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
