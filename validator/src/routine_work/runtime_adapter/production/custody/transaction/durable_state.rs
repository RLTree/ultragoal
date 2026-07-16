use super::*;

impl ReservationTransaction {
    pub(super) fn reserve(
        authority_root: &Path,
        spec: ReservationSpec,
        launch_root: PathBuf,
    ) -> Result<Self, RoutineError> {
        let ReservationSpec {
            binding,
            request_id,
            grant_id,
            recovery_marker,
            output_journal,
            owner,
            intents,
        } = spec;
        let (ledger, head) = FileAuthorityLedger::open_or_initialize(authority_root)?;
        let transaction = Self {
            durable: DurableSession {
                ledger,
                head: RefCell::new(head),
            },
            token: ReservationToken {
                binding,
                request_id,
                grant_id,
                recovery_marker,
                expires_tick: Cell::new(0),
                output_journal,
                intents,
            },
            launch_root,
            started: Cell::new(false),
            ambiguous: Cell::new(false),
            process_cleanup: RefCell::new(CleanupEvidence::NotRequired),
            launch_cleanup: RefCell::new(CleanupEvidence::NotRequired),
        };
        let reserved = {
            let mut head = transaction.durable.head.borrow_mut();
            transaction
                .durable
                .ledger
                .reserve(&mut head, &transaction.token, owner)?
        };
        match reserved {
            DurableWrite::Committed(expires_tick) => {
                transaction.token.expires_tick.set(expires_tick);
                Ok(transaction)
            }
            DurableWrite::Precommit(_) => {
                Err(error("routine-production-authority-publish-precommit"))
            }
            DurableWrite::Ambiguous(_) => {
                Err(error("routine-production-authority-publish-ambiguous"))
            }
        }
    }

    pub(super) fn settle(
        &self,
        outcome: DurableSettlement,
        result: &RoutineMediationResult,
        artifacts: &BTreeMap<String, String>,
    ) -> Result<(), RoutineError> {
        let state = match outcome {
            DurableSettlement::Complete => AttemptState::Complete,
            DurableSettlement::Failed => AttemptState::Failed,
            DurableSettlement::Cancelled => AttemptState::Cancelled,
            DurableSettlement::Incomplete => AttemptState::Incomplete,
        };
        let prior_head_sha256 = self.durable.head.borrow().head_sha256.clone();
        self.resolve(self.durable.ledger.settle_terminal(
            &mut self.durable.head.borrow_mut(),
            &self.token,
            store::TerminalRecord {
                state,
                result_sha256: digest_of(result)?,
                artifacts: artifacts.clone(),
                process_cleanup: self.process_cleanup.borrow().clone(),
                staged_cleanup: self.launch_cleanup.borrow().clone(),
                prior_head_sha256,
                failure_evidence: None,
            },
        )?)
    }

    pub(super) fn finish_failure(
        &self,
        evidence: &ReservationFailureEvidence,
    ) -> Result<(), RoutineError> {
        let exact = |cleanup: &CleanupEvidence| {
            matches!(
                cleanup,
                CleanupEvidence::NotRequired | CleanupEvidence::Succeeded
            )
        };
        if !self.ambiguous.get()
            && exact(&evidence.process_cleanup)
            && exact(&evidence.staged_cleanup)
        {
            let terminal = store::TerminalRecord {
                state: AttemptState::Failed,
                result_sha256: digest_of(evidence)?,
                artifacts: BTreeMap::new(),
                process_cleanup: evidence.process_cleanup.clone(),
                staged_cleanup: evidence.staged_cleanup.clone(),
                prior_head_sha256: self.durable.head.borrow().head_sha256.clone(),
                failure_evidence: Some(evidence.clone()),
            };
            return self.resolve(self.durable.ledger.settle_terminal(
                &mut self.durable.head.borrow_mut(),
                &self.token,
                terminal,
            )?);
        }
        self.record_failure(evidence)
    }

    pub(super) fn record_output_transition(
        &self,
        transition: &super::super::super::output_journal::OutputTransition,
    ) -> Result<(), RoutineError> {
        self.resolve(self.durable.ledger.record_output_transition(
            &mut self.durable.head.borrow_mut(),
            &self.token,
            transition,
        )?)
    }

    pub(super) fn validate_reserved(&self) -> Result<(), RoutineError> {
        self.durable
            .ledger
            .validate_reserved(&mut self.durable.head.borrow_mut(), &self.token)
    }

    pub(super) fn resolve<T>(&self, write: DurableWrite<T>) -> Result<T, RoutineError> {
        match write {
            DurableWrite::Committed(value) => Ok(value),
            DurableWrite::Precommit(_) => {
                Err(error("routine-production-authority-publish-precommit"))
            }
            DurableWrite::Ambiguous(_) => {
                self.ambiguous.set(true);
                Err(error("routine-production-authority-publish-ambiguous"))
            }
        }
    }

    fn record_failure(&self, evidence: &ReservationFailureEvidence) -> Result<(), RoutineError> {
        if self.ambiguous.get() {
            return Err(error("routine-production-authority-ambiguity-pending"));
        }
        transition_result(evidence, || {
            self.resolve(self.durable.ledger.record_failure(
                &mut self.durable.head.borrow_mut(),
                &self.token,
                evidence,
            )?)
        })?;
        Ok(())
    }
}
