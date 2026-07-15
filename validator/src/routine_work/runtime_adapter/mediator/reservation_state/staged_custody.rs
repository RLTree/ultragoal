use super::*;

impl AttemptReservation {
    pub(super) fn has_staged_custody(&self) -> bool {
        !self.staged.borrow().is_empty()
    }

    pub(super) fn require_staged_empty(&self) -> Result<(), RoutineError> {
        if self.has_staged_custody() {
            return Err(mediator_error(
                "mediator-staged-custody-terminal-transition-refused",
            ));
        }
        Ok(())
    }

    pub(in super::super) fn stage_and_use<T>(
        &self,
        program: &PinnedExecutable,
        use_program: impl FnOnce(&PinnedExecutable) -> Result<T, RoutineError>,
    ) -> Result<T, RoutineError> {
        self.require_open()?;
        self.require_staged_empty()?;
        let staged = self
            .durable
            .as_ref()
            .ok_or_else(|| mediator_error("mediator-staging-authority-missing"))?
            .stage_program(program)?;
        self.require_open()?;
        self.require_staged_empty()?;
        self.staged.borrow_mut().push(staged);
        let staged = self.staged.borrow();
        let program = staged
            .last()
            .map(|item| &item.executable)
            .ok_or_else(|| mediator_error("mediator-staged-program-custody-missing"))?;
        use_program(program)
    }

    pub(super) fn cleanup_staged(&self) -> Result<(), RoutineError> {
        let Some(durable) = &self.durable else {
            return if self.staged.borrow().is_empty() {
                Ok(())
            } else {
                Err(mediator_error("mediator-staging-authority-missing"))
            };
        };
        let mut staged = self.staged.borrow_mut();
        while let Some(item) = staged.last() {
            durable.cleanup_staged(item)?;
            staged.pop();
        }
        Ok(())
    }

    pub(super) fn failure_custody_transfer_required(
        &self,
        evidence: &CleanupEvidence,
    ) -> Result<bool, RoutineError> {
        match (self.has_staged_custody(), evidence) {
            (false, CleanupEvidence::Succeeded | CleanupEvidence::NotRequired) => Ok(false),
            (true, CleanupEvidence::Error(_) | CleanupEvidence::Panic(_))
                if self.durable.is_some() =>
            {
                Ok(true)
            }
            _ => Err(mediator_error(
                "mediator-reservation-failure-custody-binding-invalid",
            )),
        }
    }

    pub(super) fn transfer_staged_to_recorded_recovery(&self) {
        self.staged.borrow_mut().clear();
    }
}
