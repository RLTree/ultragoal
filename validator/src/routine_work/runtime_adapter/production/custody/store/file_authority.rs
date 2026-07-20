use super::*;
use crate::routine_work::runtime_adapter::production::DurableSettlement;
use crate::routine_work::runtime_adapter::production::custody::observations::{
    ChildObservation, IntentObservation, LaunchObservation, OwnerObservation, TerminalObservation,
};
use crate::routine_work::runtime_adapter::production::custody::transaction::ReservationSpec;
pub(in crate::routine_work::runtime_adapter::production::custody) struct DurableCustody {
    #[cfg(target_vendor = "apple")]
    inner: supported::record_authentication::FileLedger,
    #[cfg(target_vendor = "apple")]
    head: RefCell<supported::record_authentication::LocalHead>,
    token: ReservationToken,
    launch_stage: RefCell<Option<LaunchStageRecord>>,
    child: RefCell<Option<ChildLease>>,
}
impl DurableCustody {
    pub(in crate::routine_work::runtime_adapter::production::custody) fn reserve(
        root: &Path,
        spec: &ReservationSpec,
    ) -> Result<Self, RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            let (inner, mut head) =
                supported::record_authentication::FileLedger::open_or_initialize(root)?;
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
            let _ = (root, spec);
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
fn token(spec: &ReservationSpec) -> ReservationToken {
    ReservationToken {
        binding: spec.binding().clone(),
        owner: owner(spec.owner()),
        request_id: spec.request_id().to_owned(),
        grant_id: spec.grant_id().to_owned(),
        recovery_marker: spec.recovery_marker().to_owned(),
        expires_tick: Cell::new(0),
        output_journal: spec.output_journal().clone(),
        intents: spec.intents().iter().map(intent).collect(),
    }
}
fn intent(value: &IntentObservation) -> IntentBinding {
    IntentBinding {
        intent_id: value.intent_id.clone(),
        node_id: value.node_id.clone(),
        plan_order: value.plan_order,
        program_sha256: value.program_sha256.clone(),
    }
}
fn owner(value: &OwnerObservation) -> OwnerLease {
    OwnerLease {
        process_id: value.process_id,
        start_seconds: value.start_seconds,
        start_microseconds: value.start_microseconds,
        nonce_sha256: value.nonce_sha256.clone(),
    }
}
fn child(value: &ChildObservation) -> ChildLease {
    ChildLease {
        process_id: value.process_id,
        process_group_id: value.process_group_id,
        executable_sha256: value.executable_sha256.clone(),
        executable_device: value.executable_device,
        executable_inode: value.executable_inode,
        intent: intent(&value.intent),
    }
}
fn stage(value: &LaunchObservation) -> LaunchStageRecord {
    LaunchStageRecord {
        directory: launch_identity(value.directory),
        program: launch_identity(value.program),
        marker: launch_identity(value.marker),
        seal: launch_identity(value.seal),
        program_sha256: value.program_sha256.clone(),
        intent: intent(&value.intent),
    }
}
fn launch_identity(
    value: crate::routine_work::runtime_adapter::mediator::ObjectIdentity,
) -> LaunchEntryIdentity {
    LaunchEntryIdentity {
        device: value.device,
        inode: value.inode,
        mode: value.mode,
        owner: value.owner_user_id,
        links: value.links,
        length: value.length,
    }
}
fn terminal_state(outcome: DurableSettlement) -> AttemptState {
    match outcome {
        DurableSettlement::Complete => AttemptState::Complete,
        DurableSettlement::Failed => AttemptState::Failed,
        DurableSettlement::Cancelled => AttemptState::Cancelled,
        DurableSettlement::Incomplete => AttemptState::Incomplete,
    }
}
pub(crate) fn error(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::InvalidRequest, cause, None)
}
