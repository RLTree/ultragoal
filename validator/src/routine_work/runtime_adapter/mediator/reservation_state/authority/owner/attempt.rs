use super::super::super::super::*;
use super::super::binding::ReservationBinding;
use super::super::durable_binding::DurableBinding;
use super::super::failure_observation::{
    CapturedCleanup, LifecycleFailure, PanicPayload, combine, error_parts,
    finish_error as end_error, finish_observed, finish_panic as end_panic, panic_parts,
    transition_result,
};
use super::registry;
use std::cell::{Cell, RefCell};
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

struct AttemptReservation {
    binding: ReservationBinding,
    durable: DurableBinding,
    started: Cell<bool>,
    settled: Cell<bool>,
    staged: RefCell<Vec<StagedProgram>>,
}

pub(in super::super::super::super) struct ReservationAttempt<'a> {
    owner: &'a AttemptReservation,
}

pub(in super::super::super::super) use super::super::durable_binding::ReservationTerminal;

pub(in super::super::super::super) fn run_reserved<T>(
    grant: &RoutineRootGrant,
    lifecycle: impl FnOnce(&ReservationAttempt<'_>) -> Result<(T, ReservationTerminal), RoutineError>,
) -> Result<(T, Option<String>), RoutineError> {
    let owner = AttemptReservation::reserve(grant)?;
    let attempt = ReservationAttempt { owner: &owner };
    match catch_unwind(AssertUnwindSafe(|| lifecycle(&attempt))) {
        Ok(Ok((value, terminal))) => match owner.settle_terminal(terminal) {
            Ok(marker) => Ok((value, marker)),
            Err(error) => finish_error(&owner, error),
        },
        Ok(Err(error)) => finish_error(&owner, error),
        Err(payload) => finish_unwind(&owner, payload),
    }
}

pub(in super::super::super::super) fn observe_staged_transition(
    attempt: &ReservationAttempt<'_>,
    operation: impl FnOnce() -> Result<(), RoutineError>,
) -> Result<(), RoutineError> {
    let primary = catch_unwind(AssertUnwindSafe(operation));
    combine(primary, attempt.owner.capture_staged_cleanup())
        .unwrap_or_else(|failure| resume_unwind(Box::new(failure)));
    Ok(())
}

impl ReservationAttempt<'_> {
    pub(in super::super::super::super) fn reuse_only(&self) -> bool {
        self.owner.durable.reuse_only()
    }

    pub(in super::super::super::super) fn prepare_spawn(&self) -> Result<(), RoutineError> {
        self.owner.require_open()?;
        self.owner.durable.prepare_spawn()
    }

    pub(in super::super::super::super) fn mark_started(&self) -> Result<(), RoutineError> {
        self.owner.require_open()?;
        registry::mark_started(&self.owner.binding)?;
        self.owner.started.set(true);
        Ok(())
    }

    pub(in super::super::super::super) fn stage_success(
        &self,
        artifacts: &BTreeMap<String, String>,
    ) -> Result<(), RoutineError> {
        self.owner.require_ready()?;
        self.owner.durable.stage_success(artifacts)
    }

    pub(in super::super::super::super) fn retain_non_durable_authentication(
        &self,
        artifacts: &BTreeMap<String, String>,
    ) {
        if !self.owner.durable.is_present() {
            registry::retain_non_durable(artifacts);
        }
    }

    pub(in super::super::super::super) fn authenticates_artifact(
        &self,
        digest: &str,
        witness: &str,
    ) -> Result<bool, RoutineError> {
        if self.owner.durable.is_present() {
            self.owner.durable.authenticates_artifact(digest, witness)
        } else {
            Ok(registry::authenticates_non_durable(digest, witness))
        }
    }

    pub(in super::super::super::super) fn stage_and_use<T>(
        &self,
        program: &PinnedExecutable,
        use_program: impl FnOnce(&PinnedExecutable) -> Result<T, RoutineError>,
    ) -> Result<T, RoutineError> {
        self.owner.require_ready()?;
        let staged = self.owner.durable.stage_program(program)?;
        self.owner.require_ready()?;
        self.owner.staged.borrow_mut().push(staged);
        let staged = self.owner.staged.borrow();
        let program = staged
            .last()
            .map(|item| &item.executable)
            .ok_or_else(|| mediator_error("mediator-staged-program-custody-missing"))?;
        use_program(program)
    }
}

impl AttemptReservation {
    fn reserve(grant: &RoutineRootGrant) -> Result<Self, RoutineError> {
        if let Some(durable) = &grant.durable {
            durable.validate_reserved()?;
        }
        registry::reserve(grant)?;
        Ok(Self {
            binding: ReservationBinding::for_grant(grant),
            durable: DurableBinding::new(grant.durable.clone()),
            started: Cell::new(false),
            settled: Cell::new(false),
            staged: RefCell::new(Vec::new()),
        })
    }

    fn settle_terminal(
        &self,
        terminal: ReservationTerminal,
    ) -> Result<Option<String>, RoutineError> {
        self.require_ready()?;
        let durable = self.durable.settle_terminal(terminal)?;
        let marker = registry::finish_terminal(&self.binding, self.started.get(), durable);
        self.settled.set(true);
        Ok((!durable).then_some(marker).flatten())
    }

    fn cleanup_staged(&self) -> Result<(), RoutineError> {
        if !self.durable.is_present() && !self.staged.borrow().is_empty() {
            return Err(mediator_error("mediator-staging-authority-missing"));
        }
        loop {
            let staged = self.staged.borrow();
            let Some(item) = staged.last() else {
                return Ok(());
            };
            self.durable.cleanup_staged(item)?;
            drop(staged);
            self.staged.borrow_mut().pop();
        }
    }

    fn record_failure(&self, evidence: &ReservationFailureEvidence) -> Result<(), RoutineError> {
        self.require_open()?;
        if !self.binding.validates_failure(evidence, self.started.get()) {
            return Err(mediator_error(
                "mediator-reservation-failure-evidence-binding-invalid",
            ));
        }
        let transfer = self
            .durable
            .failure_transfer(self.staged.borrow().is_empty(), &evidence.staged_cleanup)?;
        registry::record_failure_and_transition(
            &self.binding,
            evidence,
            self.started.get(),
            || self.durable.record_failure(evidence),
        )?;
        if transfer {
            self.staged.borrow_mut().clear();
        }
        self.settled.set(true);
        Ok(())
    }

    fn capture_staged_cleanup(&self) -> CapturedCleanup {
        CapturedCleanup::from(catch_unwind(AssertUnwindSafe(|| self.cleanup_staged())))
    }

    fn require_open(&self) -> Result<(), RoutineError> {
        (!self.settled.get())
            .then_some(())
            .ok_or_else(|| mediator_error("mediator-reservation-terminal-already-settled"))
    }

    fn require_ready(&self) -> Result<(), RoutineError> {
        self.require_open()?;
        self.staged
            .borrow()
            .is_empty()
            .then_some(())
            .ok_or_else(|| mediator_error("mediator-staged-custody-terminal-transition-refused"))
    }
}

fn finish_unwind<T>(owner: &AttemptReservation, payload: PanicPayload) -> Result<T, RoutineError> {
    match payload.downcast::<LifecycleFailure>() {
        Ok(failure) => finish_observed(*failure, |parts| {
            let record = owner.binding.failure_evidence(
                parts.primary.clone(),
                parts.process_cleanup.clone(),
                parts.staged_cleanup.clone(),
                owner.started.get(),
            );
            record_transition(owner, &record)
        }),
        Err(payload) => finish_panic(owner, payload),
    }
}

fn finish_error<T>(owner: &AttemptReservation, error: RoutineError) -> Result<T, RoutineError> {
    let (primary, process) = error_parts(&error);
    let cleanup = owner.capture_staged_cleanup();
    let record = owner.binding.failure_evidence(
        primary,
        process,
        cleanup.evidence.clone(),
        owner.started.get(),
    );
    end_error(
        error,
        Some(cleanup.outcome),
        record_transition(owner, &record),
    )
}

fn finish_panic<T>(owner: &AttemptReservation, payload: PanicPayload) -> Result<T, RoutineError> {
    let (original, primary, process) = panic_parts(payload);
    let cleanup = owner.capture_staged_cleanup();
    let record = owner.binding.failure_evidence(
        primary,
        process,
        cleanup.evidence.clone(),
        owner.started.get(),
    );
    end_panic(original, record_transition(owner, &record))
}

fn record_transition(
    owner: &AttemptReservation,
    record: &ReservationFailureEvidence,
) -> Result<(), RoutineError> {
    transition_result(record, || owner.record_failure(record))
}
