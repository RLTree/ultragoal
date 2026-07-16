use super::*;
use crate::routine_work::runtime_adapter::mediator::{
    IntentExecutionRequest, PinnedExecutable, StagedProgram, StartedProcessIdentity,
};
use crate::routine_work::runtime_adapter::production::output_journal::OutputTransition;
use std::cell::Cell;

pub(super) struct ReservationOwner {
    ledger: FileAuthorityLedger,
    head: RefCell<LocalHead>,
    token: ReservationToken,
    started: Cell<bool>,
    ambiguity: RefCell<Option<DurableAmbiguity>>,
    launch_cleaned: Cell<bool>,
}

pub(super) struct LaunchHandle(LaunchStageRecord);
pub(super) struct ChildHandle(ChildLease);

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
        let owner = OwnerLease {
            process_id,
            start_seconds,
            start_microseconds,
            nonce_sha256,
        };
        let intents = request.intents().iter().map(request_intent).collect();
        let (ledger, head) = FileAuthorityLedger::open_or_initialize(authority_root)?;
        let value = Self {
            ledger,
            head: RefCell::new(head),
            token: ReservationToken {
                binding,
                request_id: request.request_id().to_owned(),
                grant_id,
                recovery_marker,
                expires_tick: Cell::new(0),
                output_journal,
                intents,
            },
            started: Cell::new(false),
            ambiguity: RefCell::new(None),
            launch_cleaned: Cell::new(false),
        };
        let reserved = value
            .ledger
            .reserve(&mut value.head.borrow_mut(), &value.token, owner)?;
        match reserved {
            DurableWrite::Committed(expires_tick) => {
                value.token.expires_tick.set(expires_tick);
                Ok(value)
            }
            DurableWrite::Precommit(_) => {
                Err(error("routine-production-authority-publish-precommit"))
            }
            DurableWrite::Ambiguous(_, ambiguity) => {
                *value.ambiguity.borrow_mut() = Some(ambiguity);
                Err(error("routine-production-authority-publish-ambiguous"))
            }
        }
    }

    pub(super) fn output_journal(&self) -> &OutputProvisionJournal {
        &self.token.output_journal
    }

    pub(super) fn failure_binding(&self) -> FailureBinding<'_> {
        FailureBinding {
            protocol_id: &self.token.binding.protocol_id,
            grant_id: &self.token.grant_id,
            recovery_marker: &self.token.recovery_marker,
        }
    }

    pub(super) fn started(&self) -> bool {
        self.started.get()
    }

    pub(super) fn validate_reserved(&self) -> Result<(), RoutineError> {
        self.ledger
            .validate_reserved(&mut self.head.borrow_mut(), &self.token)
    }

    pub(super) fn note_output_ambiguity(&self) {
        let head = self.head.borrow().head_sha256.clone();
        self.ambiguity.borrow_mut().get_or_insert(DurableAmbiguity {
            previous_head_sha256: head.clone(),
            proposed_head_sha256: head,
        });
    }

    pub(super) fn record_output(&self, transition: &OutputTransition) -> Result<(), RoutineError> {
        self.resolve(self.ledger.record_output_transition(
            &mut self.head.borrow_mut(),
            &self.token,
            transition,
        )?)
    }

    pub(super) fn record_stage(
        &self,
        staged: &StagedProgram,
        intent: &IntentExecutionRequest,
    ) -> Result<LaunchHandle, RoutineError> {
        let record = launch_record(staged, intent);
        self.resolve(self.ledger.record_launch_stage(
            &mut self.head.borrow_mut(),
            &self.token,
            &record,
        )?)?;
        Ok(LaunchHandle(record))
    }

    pub(super) fn record_started(
        &self,
        started: StartedProcessIdentity,
        executable: &PinnedExecutable,
        intent: &IntentExecutionRequest,
    ) -> Result<ChildHandle, RoutineError> {
        let child = ChildLease {
            process_id: started.process_id(),
            process_group_id: started.process_group_id(),
            executable_sha256: executable.sha256.clone(),
            executable_device: executable.identity.device,
            executable_inode: executable.identity.inode,
            intent: observed_intent(intent, &executable.sha256),
        };
        match self
            .ledger
            .prepare_spawn(&mut self.head.borrow_mut(), &self.token, child.clone())?
        {
            DurableWrite::Committed(()) => {
                self.started.set(true);
                Ok(ChildHandle(child))
            }
            DurableWrite::Precommit(()) => {
                Err(error("routine-production-authority-publish-precommit"))
            }
            DurableWrite::Ambiguous((), ambiguity) => {
                self.started.set(true);
                *self.ambiguity.borrow_mut() = Some(ambiguity);
                Err(error("routine-production-authority-publish-ambiguous"))
            }
        }
    }

    pub(super) fn record_reaped(&self, child: ChildHandle) -> Result<(), RoutineError> {
        self.resolve(self.ledger.record_process_reaped(
            &mut self.head.borrow_mut(),
            &self.token,
            &child.0,
        )?)
    }

    pub(super) fn record_stage_cleaned(&self, stage: LaunchHandle) -> Result<(), RoutineError> {
        self.resolve(self.ledger.record_launch_cleaned(
            &mut self.head.borrow_mut(),
            &self.token,
            &stage.0,
        )?)?;
        self.launch_cleaned.set(true);
        Ok(())
    }

    pub(super) fn settle(
        &self,
        outcome: DurableSettlement,
        result: &RoutineMediationResult,
        artifacts: &BTreeMap<String, String>,
    ) -> Result<(), RoutineError> {
        let state = terminal_state(outcome);
        let terminal = TerminalRecord {
            state,
            result_sha256: digest_of(result)?,
            artifacts: artifacts.clone(),
            process_cleanup: cleanup_state(self.started.get()),
            staged_cleanup: cleanup_state(self.launch_cleaned.get()),
            prior_head_sha256: self.head.borrow().head_sha256.clone(),
            failure_evidence: None,
        };
        self.resolve(self.ledger.settle_terminal(
            &mut self.head.borrow_mut(),
            &self.token,
            terminal,
        )?)
    }

    pub(super) fn finish_failure(
        &self,
        evidence: &ReservationFailureEvidence,
    ) -> Result<(), RoutineError> {
        if self.ambiguity.borrow().is_some() || !cleanup_exact(evidence) {
            return observed_transition(
                evidence,
                catch_unwind(AssertUnwindSafe(|| {
                    self.resolve(self.ledger.record_failure(
                        &mut self.head.borrow_mut(),
                        &self.token,
                        evidence,
                    )?)
                })),
            );
        }
        let terminal = TerminalRecord {
            state: AttemptState::Failed,
            result_sha256: digest_of(evidence)?,
            artifacts: BTreeMap::new(),
            process_cleanup: evidence.process_cleanup.clone(),
            staged_cleanup: evidence.staged_cleanup.clone(),
            prior_head_sha256: self.head.borrow().head_sha256.clone(),
            failure_evidence: Some(evidence.clone()),
        };
        self.resolve(self.ledger.settle_terminal(
            &mut self.head.borrow_mut(),
            &self.token,
            terminal,
        )?)
    }

    fn resolve<T>(&self, value: DurableWrite<T>) -> Result<T, RoutineError> {
        match value {
            DurableWrite::Committed(value) => Ok(value),
            DurableWrite::Precommit(_) => {
                Err(error("routine-production-authority-publish-precommit"))
            }
            DurableWrite::Ambiguous(_, ambiguity) => {
                *self.ambiguity.borrow_mut() = Some(ambiguity);
                Err(error("routine-production-authority-publish-ambiguous"))
            }
        }
    }
}
