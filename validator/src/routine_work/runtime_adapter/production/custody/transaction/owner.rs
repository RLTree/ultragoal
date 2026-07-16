use super::*;
use crate::routine_work::runtime_adapter::mediator::{IntentExecutionRequest, StagedProgram};
use crate::routine_work::runtime_adapter::production::output_journal::OutputTransition;
use std::cell::Cell;

pub(super) struct ReservationOwner {
    ledger: DurableCustody,
    started: Cell<bool>,
    ambiguity: Cell<bool>,
    launch_cleaned: Cell<bool>,
}

pub(super) struct LaunchHandle;
pub(super) struct ChildHandle;

pub(in crate::routine_work::runtime_adapter::production::custody) struct ReservationSpec {
    binding: AuthorityBinding,
    request_id: String,
    grant_id: String,
    recovery_marker: String,
    owner: OwnerObservation,
    output_journal: OutputProvisionJournal,
    intents: Vec<IntentObservation>,
}

impl ReservationSpec {
    pub(in crate::routine_work::runtime_adapter::production::custody) fn binding(
        &self,
    ) -> &AuthorityBinding {
        &self.binding
    }
    pub(in crate::routine_work::runtime_adapter::production::custody) fn request_id(&self) -> &str {
        &self.request_id
    }
    pub(in crate::routine_work::runtime_adapter::production::custody) fn grant_id(&self) -> &str {
        &self.grant_id
    }
    pub(in crate::routine_work::runtime_adapter::production::custody) fn recovery_marker(
        &self,
    ) -> &str {
        &self.recovery_marker
    }
    pub(in crate::routine_work::runtime_adapter::production::custody) fn owner(
        &self,
    ) -> &OwnerObservation {
        &self.owner
    }
    pub(in crate::routine_work::runtime_adapter::production::custody) fn output_journal(
        &self,
    ) -> &OutputProvisionJournal {
        &self.output_journal
    }
    pub(in crate::routine_work::runtime_adapter::production::custody) fn intents(
        &self,
    ) -> &[IntentObservation] {
        &self.intents
    }
}

impl ReservationOwner {
    pub(super) fn reserve(
        authority_root: &Path,
        request: &RoutineEffectRequest,
        binding: AuthorityBinding,
        output_journal: OutputProvisionJournal,
    ) -> Result<Self, RoutineError> {
        let scopes = allowed_output_scopes(request);
        let session_id = random_session_id(&binding)?;
        let grant_id = reservation_grant_id(&session_id, request, &binding, &scopes)?;
        let recovery_marker =
            recovery_identity(&grant_id, request.protocol_id(), request.request_id());
        let (process_id, start_seconds, start_microseconds, nonce_sha256) =
            owner_process_identity()?;
        let owner = OwnerObservation {
            process_id,
            start_seconds,
            start_microseconds,
            nonce_sha256,
        };
        let intents = request.intents().iter().map(request_intent).collect();
        let spec = ReservationSpec {
            binding,
            request_id: request.request_id().to_owned(),
            grant_id,
            recovery_marker,
            owner,
            output_journal,
            intents,
        };
        let ledger = DurableCustody::reserve(authority_root, &spec)?;
        Ok(Self {
            ledger,
            started: Cell::new(false),
            ambiguity: Cell::new(false),
            launch_cleaned: Cell::new(false),
        })
    }

    pub(super) fn output_journal(&self) -> &OutputProvisionJournal {
        self.ledger.output_journal()
    }

    pub(super) fn failure_binding(&self) -> FailureBinding<'_> {
        let (protocol_id, grant_id, recovery_marker) = self.ledger.failure_binding();
        FailureBinding {
            protocol_id,
            grant_id,
            recovery_marker,
        }
    }

    pub(super) fn started(&self) -> bool {
        self.started.get()
    }

    pub(super) fn validate_reserved(&self) -> Result<(), RoutineError> {
        self.ledger.validate_reserved()
    }

    pub(super) fn note_output_ambiguity(&self) {
        self.ambiguity.set(true);
    }

    pub(super) fn record_output(&self, transition: &OutputTransition) -> Result<(), RoutineError> {
        self.resolve(self.ledger.record_output_transition(transition)?)
    }

    pub(super) fn record_stage(
        &self,
        staged: &StagedProgram,
        intent: &IntentExecutionRequest,
    ) -> Result<LaunchHandle, RoutineError> {
        self.resolve(
            self.ledger
                .record_launch_stage(&launch_observation(staged, intent))?,
        )?;
        Ok(LaunchHandle)
    }

    pub(super) fn record_started(
        &self,
        started: crate::routine_work::runtime_adapter::mediator::StartedProcessIdentity,
        executable: &crate::routine_work::runtime_adapter::mediator::PinnedExecutable,
        intent: &IntentExecutionRequest,
    ) -> Result<ChildHandle, RoutineError> {
        match self
            .ledger
            .prepare_spawn(&child_observation(started, executable, intent))?
        {
            DurableWrite::Committed(()) => {
                self.started.set(true);
                Ok(ChildHandle)
            }
            DurableWrite::Precommit(()) => {
                Err(error("routine-production-authority-publish-precommit"))
            }
            DurableWrite::Ambiguous((), ambiguity) => {
                self.started.set(true);
                let _ = ambiguity;
                self.ambiguity.set(true);
                Err(error("routine-production-authority-publish-ambiguous"))
            }
        }
    }

    pub(super) fn record_reaped(&self, child: ChildHandle) -> Result<(), RoutineError> {
        let _ = child;
        self.resolve(self.ledger.record_process_reaped()?)
    }

    pub(super) fn record_stage_cleaned(&self, stage: LaunchHandle) -> Result<(), RoutineError> {
        let _ = stage;
        self.resolve(self.ledger.record_launch_cleaned()?)?;
        self.launch_cleaned.set(true);
        Ok(())
    }

    pub(super) fn settle(
        &self,
        outcome: DurableSettlement,
        result: &RoutineMediationResult,
        artifacts: &BTreeMap<String, String>,
    ) -> Result<(), RoutineError> {
        let terminal = TerminalObservation {
            outcome,
            result_sha256: digest_of(result)?,
            artifacts: artifacts.clone(),
            process_cleanup: cleanup_state(self.started.get()),
            staged_cleanup: cleanup_state(self.launch_cleaned.get()),
            failure_evidence: None,
        };
        self.resolve(self.ledger.settle_terminal(terminal)?)
    }

    pub(super) fn finish_failure(
        &self,
        evidence: &ReservationFailureEvidence,
    ) -> Result<(), RoutineError> {
        if self.ambiguity.get() || !cleanup_exact(evidence) {
            return observed_transition(
                evidence,
                catch_unwind(AssertUnwindSafe(|| {
                    self.resolve(self.ledger.record_failure(evidence)?)
                })),
            );
        }
        let terminal = TerminalObservation {
            outcome: DurableSettlement::Failed,
            result_sha256: digest_of(evidence)?,
            artifacts: BTreeMap::new(),
            process_cleanup: evidence.process_cleanup.clone(),
            staged_cleanup: evidence.staged_cleanup.clone(),
            failure_evidence: Some(evidence.clone()),
        };
        self.resolve(self.ledger.settle_terminal(terminal)?)
    }

    fn resolve<T>(&self, value: DurableWrite<T>) -> Result<T, RoutineError> {
        match value {
            DurableWrite::Committed(value) => Ok(value),
            DurableWrite::Precommit(_) => {
                Err(error("routine-production-authority-publish-precommit"))
            }
            DurableWrite::Ambiguous(_, ambiguity) => {
                let _ = ambiguity;
                self.ambiguity.set(true);
                Err(error("routine-production-authority-publish-ambiguous"))
            }
        }
    }
}
