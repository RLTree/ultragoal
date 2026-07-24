use super::observation::{child, stage, terminal_state, token};
use super::*;
use crate::routine_work::runtime_adapter::mediator::RoutineContinuationOutcome;
use crate::routine_work::runtime_adapter::production::custody::RoutineCustodyCapability;
use crate::routine_work::runtime_adapter::production::custody::observations::{
    ChildObservation, LaunchObservation, TerminalObservation,
};
use crate::routine_work::runtime_adapter::production::custody::transaction::ReservationSpec;
use std::path::Path;
pub(in crate::routine_work::runtime_adapter::production::custody) struct DurableCustody {
    #[cfg(target_vendor = "apple")]
    inner: supported::authentication::FileLedger,
    #[cfg(target_vendor = "apple")]
    head: RefCell<supported::authentication::LocalHead>,
    token: ReservationToken,
    launch_stage: RefCell<Option<LaunchStageRecord>>,
    child: RefCell<Option<ChildLease>>,
}
impl DurableCustody {
    pub(in crate::routine_work::runtime_adapter::production::custody) fn authenticate_public_checkpoint(
        custody: &RoutineCustodyCapability,
        target: &Path,
        context_id: &str,
        candidate_id: &str,
        plan_id: &str,
        snapshot_id: &str,
        continuation: &str,
        recovery_marker: &str,
        predecessor_continuations: &[String],
        attempt_grant: &str,
        authenticated_ledger_head: &str,
        state: &str,
        terminal_outcome: Option<&str>,
        allow_stale_head: bool,
    ) -> Result<(), RoutineError> {
        let root = custody.authority_root();
        #[cfg(target_vendor = "apple")]
        {
            let (inner, mut head) = supported::authentication::FileLedger::open_existing(root)?;
            inner.authenticate_public_checkpoint(
                &mut head,
                target,
                context_id,
                candidate_id,
                plan_id,
                snapshot_id,
                continuation,
                recovery_marker,
                predecessor_continuations,
                attempt_grant,
                authenticated_ledger_head,
                state,
                terminal_outcome,
                allow_stale_head,
            )
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (
                custody,
                target,
                context_id,
                candidate_id,
                plan_id,
                snapshot_id,
                continuation,
                recovery_marker,
                predecessor_continuations,
                attempt_grant,
                authenticated_ledger_head,
                state,
                terminal_outcome,
                allow_stale_head,
            );
            Err(error("routine-production-authority-host-unsupported"))
        }
    }

    pub(in crate::routine_work::runtime_adapter::production::custody) fn reconcile_reserved(
        custody: &RoutineCustodyCapability,
        binding: &AuthorityBinding,
        attempt_grant: &str,
        expected_head: &str,
    ) -> Result<RoutineContinuationOutcome, RoutineError> {
        let root = custody.authority_root();
        #[cfg(target_vendor = "apple")]
        {
            let (inner, mut head) =
                supported::authentication::FileLedger::open_or_initialize(root)?;
            match inner.reconcile_reserved(&mut head, binding, attempt_grant, expected_head)? {
                DurableWrite::Committed(ContinuationDisposition::Complete(result)) => {
                    Ok(RoutineContinuationOutcome::Complete(result))
                }
                DurableWrite::Committed(ContinuationDisposition::Reserved) => {
                    Ok(RoutineContinuationOutcome::Reserved {
                        authenticated_head: head.head_sha256().to_owned(),
                    })
                }
                DurableWrite::Precommit(_) => {
                    Err(error("routine-production-authority-publish-precommit"))
                }
                DurableWrite::Ambiguous(_, _) => {
                    Err(error("routine-production-authority-publish-ambiguous"))
                }
            }
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (custody, binding, attempt_grant, expected_head);
            Err(error("routine-production-authority-host-unsupported"))
        }
    }
    pub(in crate::routine_work::runtime_adapter::production::custody) fn reserve(
        custody: &RoutineCustodyCapability,
        spec: &ReservationSpec,
    ) -> Result<Self, RoutineError> {
        let root = custody.authority_root();
        #[cfg(target_vendor = "apple")]
        {
            let (inner, mut head) =
                supported::authentication::FileLedger::open_or_initialize(root)?;
            let token = token(spec);
            let write = inner.reserve(&mut head, &token)?;
            match write {
                DurableWrite::Committed(expires_tick) => {
                    token.expires_tick.set(expires_tick);
                    Ok(Self {
                        inner,
                        head: RefCell::new(head),
                        token,
                        launch_stage: RefCell::new(None),
                        child: RefCell::new(None),
                    })
                }
                DurableWrite::Precommit(_) => {
                    Err(error("routine-production-authority-publish-precommit"))
                }
                DurableWrite::Ambiguous(_, _) => {
                    Err(error("routine-production-authority-publish-ambiguous"))
                }
            }
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (custody, spec);
            Err(error("routine-production-authority-host-unsupported"))
        }
    }
    pub(in crate::routine_work::runtime_adapter::production::custody) fn failure_binding(
        &self,
    ) -> (&str, &str, &str) {
        (
            &self.token.binding.protocol_id,
            &self.token.grant_id,
            &self.token.recovery_marker,
        )
    }
    pub(in crate::routine_work::runtime_adapter::production::custody) fn authenticated_head(
        &self,
    ) -> String {
        self.head.borrow().head_sha256().to_owned()
    }
    pub(in crate::routine_work::runtime_adapter::production::custody) fn output_journal(
        &self,
    ) -> &OutputProvisionJournal {
        &self.token.output_journal
    }
    pub(in crate::routine_work::runtime_adapter::production::custody) fn validate_reserved(
        &self,
    ) -> Result<(), RoutineError> {
        self.inner
            .validate_reserved(&mut self.head.borrow_mut(), &self.token)
    }
    pub(in crate::routine_work::runtime_adapter::production::custody) fn prepare_spawn(
        &self,
        observation: &ChildObservation,
    ) -> Result<DurableWrite<()>, RoutineError> {
        let child = child(observation);
        let write = self
            .inner
            .prepare_spawn(&mut self.head.borrow_mut(), &self.token, &child)?;
        if !matches!(write, DurableWrite::Precommit(_)) {
            *self.child.borrow_mut() = Some(child);
        }
        Ok(write)
    }
    pub(in crate::routine_work::runtime_adapter::production::custody) fn record_launch_stage(
        &self,
        observation: &LaunchObservation,
    ) -> Result<DurableWrite<()>, RoutineError> {
        let stage = stage(observation);
        let write =
            self.inner
                .record_launch_stage(&mut self.head.borrow_mut(), &self.token, &stage)?;
        if !matches!(write, DurableWrite::Precommit(_)) {
            *self.launch_stage.borrow_mut() = Some(stage);
        }
        Ok(write)
    }
    pub(in crate::routine_work::runtime_adapter::production::custody) fn record_launch_cleaned(
        &self,
    ) -> Result<DurableWrite<()>, RoutineError> {
        let stage = self
            .launch_stage
            .borrow()
            .clone()
            .ok_or_else(|| error("routine-production-launch-custody-missing"))?;
        let write =
            self.inner
                .record_launch_cleaned(&mut self.head.borrow_mut(), &self.token, &stage)?;
        if matches!(write, DurableWrite::Committed(())) {
            self.launch_stage.borrow_mut().take();
        }
        Ok(write)
    }
    pub(in crate::routine_work::runtime_adapter::production::custody) fn record_process_reaped(
        &self,
    ) -> Result<DurableWrite<()>, RoutineError> {
        let child = self
            .child
            .borrow()
            .clone()
            .ok_or_else(|| error("routine-production-child-custody-missing"))?;
        let write =
            self.inner
                .record_process_reaped(&mut self.head.borrow_mut(), &self.token, &child)?;
        if matches!(write, DurableWrite::Committed(())) {
            self.child.borrow_mut().take();
        }
        Ok(write)
    }
    pub(in crate::routine_work::runtime_adapter::production::custody) fn record_output_transition(
        &self,
        transition: &super::super::output_journal::OutputTransition,
    ) -> Result<DurableWrite<()>, RoutineError> {
        match transition {
            super::super::output_journal::OutputTransition::Staged {
                relative_path,
                identity,
            } => self.inner.record_output_staged(
                &mut self.head.borrow_mut(),
                &self.token,
                relative_path,
                *identity,
            ),
            super::super::output_journal::OutputTransition::Published {
                relative_path,
                identity,
            } => self.inner.record_output_component(
                &mut self.head.borrow_mut(),
                &self.token,
                relative_path,
                *identity,
            ),
        }
    }
    pub(in crate::routine_work::runtime_adapter::production::custody) fn settle_terminal(
        &self,
        observation: TerminalObservation,
    ) -> Result<DurableWrite<()>, RoutineError> {
        let terminal = TerminalRecord {
            state: terminal_state(observation.outcome),
            result_sha256: observation.result_sha256,
            artifacts: observation.artifacts,
            mediation: observation.mediation,
            process_cleanup: observation.process_cleanup,
            staged_cleanup: observation.staged_cleanup,
            prior_head_sha256: self.head.borrow().head_sha256().to_owned(),
            failure_evidence: observation.failure_evidence,
        };
        self.inner
            .settle_terminal(&mut self.head.borrow_mut(), &self.token, terminal)
    }
    pub(in crate::routine_work::runtime_adapter::production::custody) fn record_failure(
        &self,
        evidence: &ReservationFailureEvidence,
    ) -> Result<DurableWrite<()>, RoutineError> {
        self.inner
            .record_failure(&mut self.head.borrow_mut(), &self.token, evidence)
    }
}
pub(crate) fn error(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::InvalidRequest, cause, None)
}
