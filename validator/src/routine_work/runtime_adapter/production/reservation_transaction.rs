use super::launch_recovery::recover_staged;
use super::launch_snapshot::launch_root;
use super::production_issuance::{
    output_journal as select_output_journal, reservation_spec, reuse_claims, validate_recovery,
};
use super::production_mediation::{authority_binding, error};
use super::reservation_failure::{CapturedCleanup, combine, finish_transaction, transition_result};
use super::*;
use std::cell::{Cell, RefCell};
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

struct ReservationTransaction {
    ledger: Arc<FileAuthorityLedger>,
    token: ReservationToken,
    launch_root: PathBuf,
    started: Cell<bool>,
    settled: Cell<bool>,
    staged: RefCell<Vec<super::super::mediator::StagedProgram>>,
}

pub(crate) struct RoutineExecutionCapability<'a> {
    owner: &'a ReservationTransaction,
}

pub(super) fn mediate_reserved_effect(
    authority_root: &Path,
    context: &LiveContext,
    plan: &RoutinePlan,
    request: RoutineEffectRequest,
    cancellation: RoutineCancellation,
    reuse: PreflightedProductionReuse,
    publisher: &dyn RoutineArtifactPublisher,
) -> Result<RoutineMediationResult, RoutineError> {
    preflight_production_request(context, plan, &request)?;
    let launch_root = launch_root(authority_root)?;
    let ledger = Arc::new(if reuse.is_empty() {
        FileAuthorityLedger::open_or_initialize(authority_root)?
    } else {
        FileAuthorityLedger::open_existing(authority_root)?
    });
    let binding = authority_binding(&request)?;
    let recovery = match ledger.pending_recovery(&binding)? {
        Some(pending) => {
            recover_staged(
                &launch_root,
                &pending.grant_id,
                &pending.marker,
                request.intents(),
            )?;
            Some(validate_recovery(pending)?)
        }
        None => None,
    };
    let (reuse, claims) = reuse.into_parts();
    let recovery_for = recovery.as_ref().map(|item| item.marker.clone());
    let reuse_only = recovery_for.is_none() && !reuse.is_empty();
    if !claims.is_empty() && !(reuse_only || recovery_for.is_some()) {
        return Err(error("mediator-production-reuse-not-authenticated"));
    }
    let preauthorization = reuse_only
        .then(|| ledger.preauthorize_reuse(&binding, reuse_claims(claims)))
        .transpose()?;
    let journal = select_output_journal(context, &request, recovery.as_ref())?;
    let spec = reservation_spec(
        &request,
        binding,
        recovery_for,
        reuse_only,
        preauthorization,
        journal,
    )?;
    let token = ledger.reserve(spec)?;
    let owner = ReservationTransaction::new(ledger, token, launch_root);
    let capability = RoutineExecutionCapability { owner: &owner };
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        owner.ledger.validate_reserved(&owner.token)?;
        let output =
            super::output_journal::apply(&owner.ledger, &owner.token, context.worktree_root())?;
        super::output_journal::resolve_application(&owner.ledger, &owner.token, output)?;
        let observed = super::super::mediator::mediate_effect_observed(
            context,
            plan,
            request,
            cancellation,
            reuse,
            &capability,
        )?;
        let (result, settlement, artifacts, authenticated) = observed.into_parts();
        owner.require_ready()?;
        let settlement_artifacts = if settlement == DurableSettlement::Complete {
            if owner.token.reuse_only {
                BTreeMap::new()
            } else {
                owner.ledger.stage_success(&owner.token, &authenticated)?;
                publisher.publish(&artifacts)?;
                authenticated
            }
        } else {
            BTreeMap::new()
        };
        owner.settle(settlement, &settlement_artifacts)?;
        Ok(result)
    }));
    finish_transaction(
        outcome,
        &owner.token,
        owner.started.get(),
        || owner.capture_staged_cleanup(),
        |record| owner.record_failure(record),
    )
}

impl ReservationTransaction {
    fn new(
        ledger: Arc<FileAuthorityLedger>,
        token: ReservationToken,
        launch_root: PathBuf,
    ) -> Self {
        Self {
            ledger,
            token,
            launch_root,
            started: Cell::new(false),
            settled: Cell::new(false),
            staged: RefCell::new(Vec::new()),
        }
    }

    fn settle(
        &self,
        outcome: DurableSettlement,
        artifacts: &BTreeMap<String, String>,
    ) -> Result<(), RoutineError> {
        self.require_ready()?;
        let state = match outcome {
            DurableSettlement::Complete => AttemptState::Complete,
            DurableSettlement::Failed => AttemptState::Failed,
            DurableSettlement::Cancelled => AttemptState::Cancelled,
            DurableSettlement::Incomplete => AttemptState::Incomplete,
        };
        self.ledger.settle(&self.token, state, artifacts)?;
        self.settled.set(true);
        Ok(())
    }

    fn cleanup_staged(&self) -> Result<(), RoutineError> {
        loop {
            let staged = self.staged.borrow();
            let Some(item) = staged.last() else {
                return Ok(());
            };
            super::launch_custody::cleanup_staged(item)?;
            drop(staged);
            self.staged.borrow_mut().pop();
        }
    }

    fn capture_staged_cleanup(&self) -> CapturedCleanup {
        CapturedCleanup::from(catch_unwind(AssertUnwindSafe(|| self.cleanup_staged())))
    }

    fn record_failure(&self, evidence: &ReservationFailureEvidence) -> Result<(), RoutineError> {
        self.require_open()?;
        transition_result(evidence, || {
            self.ledger.record_failure(&self.token, evidence)
        })?;
        self.settled.set(true);
        Ok(())
    }

    fn require_open(&self) -> Result<(), RoutineError> {
        (!self.settled.get())
            .then_some(())
            .ok_or_else(|| error("routine-production-reservation-already-settled"))
    }

    fn require_ready(&self) -> Result<(), RoutineError> {
        self.require_open()?;
        self.staged
            .borrow()
            .is_empty()
            .then_some(())
            .ok_or_else(|| error("routine-production-staged-custody-not-empty"))
    }
}

impl RoutineExecutionCapability<'_> {
    pub(crate) fn reuse_only(&self) -> bool {
        self.owner.token.reuse_only
    }

    pub(crate) fn authenticates_artifact(
        &self,
        digest: &str,
        witness: &str,
    ) -> Result<bool, RoutineError> {
        self.owner
            .ledger
            .authenticates(&self.owner.token, digest, witness)
    }

    pub(crate) fn prepare_spawn(&self) -> Result<(), RoutineError> {
        self.owner.require_open()?;
        self.owner.ledger.prepare_spawn(&self.owner.token)?;
        self.owner.started.set(true);
        Ok(())
    }

    pub(crate) fn stage_and_use<T>(
        &self,
        program: &super::super::mediator::PinnedExecutable,
        use_program: impl FnOnce(&super::super::mediator::PinnedExecutable) -> Result<T, RoutineError>,
    ) -> Result<T, RoutineError> {
        self.owner.require_ready()?;
        let staged = super::launch_snapshot::stage_program(
            &self.owner.launch_root,
            &self.owner.token,
            program,
        )?;
        self.owner.staged.borrow_mut().push(staged);
        let staged = self.owner.staged.borrow();
        let executable = &staged
            .last()
            .ok_or_else(|| error("routine-staged-custody-missing"))?
            .executable;
        use_program(executable)
    }

    pub(crate) fn observe_staged_transition(
        &self,
        operation: impl FnOnce() -> Result<(), RoutineError>,
    ) -> Result<(), RoutineError> {
        let primary = catch_unwind(AssertUnwindSafe(operation));
        combine(primary, self.owner.capture_staged_cleanup())
            .unwrap_or_else(|failure| resume_unwind(Box::new(failure)));
        Ok(())
    }
}
