use super::*;

pub(super) struct DurableBinding(Option<Arc<dyn DurableAttemptAuthority>>);

impl DurableBinding {
    pub(super) fn new(authority: Option<Arc<dyn DurableAttemptAuthority>>) -> Self {
        Self(authority)
    }

    pub(super) fn is_present(&self) -> bool {
        self.0.is_some()
    }

    pub(super) fn reuse_only(&self) -> bool {
        self.0.as_ref().is_some_and(|item| item.reuse_only())
    }

    pub(super) fn prepare_spawn(&self) -> Result<(), RoutineError> {
        self.0.as_ref().map_or(Ok(()), |item| item.prepare_spawn())
    }

    pub(super) fn stage_success(
        &self,
        artifacts: &BTreeMap<String, String>,
    ) -> Result<(), RoutineError> {
        self.0
            .as_ref()
            .map_or(Ok(()), |item| item.stage_success(artifacts))
    }

    pub(super) fn settle(
        &self,
        outcome: DurableSettlement,
        artifacts: &BTreeMap<String, String>,
    ) -> Result<bool, RoutineError> {
        let Some(item) = &self.0 else {
            return Ok(false);
        };
        item.settle(outcome, artifacts)?;
        Ok(true)
    }

    pub(super) fn authenticates_artifact(
        &self,
        digest: &str,
        witness: &str,
    ) -> Result<Option<bool>, RoutineError> {
        self.0
            .as_ref()
            .map(|item| item.authenticates_artifact(digest, witness))
            .transpose()
    }

    pub(super) fn stage_program(
        &self,
        program: &PinnedExecutable,
    ) -> Result<StagedProgram, RoutineError> {
        self.0
            .as_ref()
            .ok_or_else(|| mediator_error("mediator-staging-authority-missing"))?
            .stage_program(program)
    }

    pub(super) fn cleanup_staged(&self, staged: &StagedProgram) -> Result<(), RoutineError> {
        self.0
            .as_ref()
            .ok_or_else(|| mediator_error("mediator-staging-authority-missing"))?
            .cleanup_staged(staged)
    }

    pub(super) fn record_failure(
        &self,
        evidence: &ReservationFailureEvidence,
    ) -> Result<bool, RoutineError> {
        let Some(item) = &self.0 else {
            return Ok(false);
        };
        item.record_failure(evidence)?;
        Ok(true)
    }
}
