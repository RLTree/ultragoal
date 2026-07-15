use super::*;

impl AttemptReservation {
    pub(crate) fn stage_program(
        &self,
        program: &PinnedExecutable,
    ) -> Result<StagedProgram, RoutineError> {
        self.durable
            .as_ref()
            .ok_or_else(|| mediator_error("mediator-staging-authority-missing"))?
            .stage_program(program)
    }

    pub(crate) fn retain_staged(&self, staged: StagedProgram) {
        self.staged.borrow_mut().push(staged);
    }

    pub(crate) fn cleanup_staged(&self) -> Result<(), RoutineError> {
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
}
