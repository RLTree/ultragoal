use super::super::launch_custody::{LaunchBinding, cleanup_staged, launch_root, stage_program};
use super::super::production_mediation::{
    allowed_output_scopes, authority_binding, error, owner_process_identity, random_session_id,
    reservation_grant_id, validate_observed_mediation,
};
use super::super::reservation_failure::{
    CapturedCleanup, FailureBinding, finish_execution, finish_transaction, transition_result,
};
#[path = "transaction/bindings.rs"]
mod bindings;
#[path = "transaction/durable_state.rs"]
mod durable_state;
#[path = "transaction/intent_execution.rs"]
mod intent_execution;
#[path = "store/mod.rs"]
mod store;

use super::*;
use std::cell::{Cell, RefCell};
use std::panic::{AssertUnwindSafe, catch_unwind};
use store::{
    AttemptState, ChildLease, DurableWrite, FileAuthorityLedger, LocalHead, OwnerLease,
    ReservationSpec, ReservationToken,
};
pub(in crate::routine_work::runtime_adapter::production) use store::{
    AuthorityBinding, OutputComponentJournal, OutputDirectoryIdentity, OutputProvisionJournal,
    OutputStageAmbiguity,
};

struct ReservationTransaction {
    durable: DurableSession,
    token: ReservationToken,
    launch_root: PathBuf,
    started: Cell<bool>,
    ambiguous: Cell<bool>,
    process_cleanup: RefCell<CleanupEvidence>,
    launch_cleanup: RefCell<CleanupEvidence>,
}

struct DurableSession {
    ledger: FileAuthorityLedger,
    head: RefCell<LocalHead>,
}

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
    let spec = bindings::reservation_spec(&request, binding, journal)?;
    let owner = ReservationTransaction::reserve(authority_root, spec, launch_root)?;
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        owner.validate_reserved()?;
        let mut output = super::super::output_journal::begin(
            &owner.token.output_journal,
            context.worktree_root(),
        )?;
        loop {
            match output.next()? {
                super::super::output_journal::OutputStep::Record(transition) => {
                    owner.record_output_transition(&transition)?;
                    output.recorded(transition)?;
                }
                super::super::output_journal::OutputStep::Complete(outcome) => {
                    if outcome.into_ambiguity().is_some() {
                        owner.ambiguous.set(true);
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
            let observation = owner.execute_intent(&intent)?;
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
        Ok(result)
    }));
    finish_transaction(
        outcome,
        FailureBinding {
            protocol_id: &owner.token.binding.protocol_id,
            grant_id: &owner.token.grant_id,
            recovery_marker: &owner.token.recovery_marker,
        },
        owner.started.get(),
        || CapturedCleanup::from(Ok(Ok(()))),
        |record| owner.finish_failure(record),
    )
}
