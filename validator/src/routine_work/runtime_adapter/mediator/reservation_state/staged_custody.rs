use super::*;
use std::cell::RefCell;

pub(super) struct StagedCustody(RefCell<Vec<StagedProgram>>);

impl StagedCustody {
    pub(super) fn new() -> Self {
        Self(RefCell::new(Vec::new()))
    }

    pub(super) fn is_empty(&self) -> bool {
        self.0.borrow().is_empty()
    }

    pub(super) fn require_empty(&self) -> Result<(), RoutineError> {
        if !self.is_empty() {
            return Err(mediator_error(
                "mediator-staged-custody-terminal-transition-refused",
            ));
        }
        Ok(())
    }

    pub(super) fn push_and_use<T>(
        &self,
        staged: StagedProgram,
        use_program: impl FnOnce(&PinnedExecutable) -> Result<T, RoutineError>,
    ) -> Result<T, RoutineError> {
        self.0.borrow_mut().push(staged);
        let staged = self.0.borrow();
        let program = staged
            .last()
            .map(|item| &item.executable)
            .ok_or_else(|| mediator_error("mediator-staged-program-custody-missing"))?;
        use_program(program)
    }

    pub(super) fn cleanup_last(
        &self,
        cleanup: impl FnOnce(&StagedProgram) -> Result<(), RoutineError>,
    ) -> Result<bool, RoutineError> {
        let staged = self.0.borrow();
        let Some(item) = staged.last() else {
            return Ok(false);
        };
        cleanup(item)?;
        drop(staged);
        self.0.borrow_mut().pop();
        Ok(true)
    }

    pub(super) fn clear_recorded(&self) {
        self.0.borrow_mut().clear();
    }

    pub(super) fn failure_transfer_required(
        &self,
        evidence: &CleanupEvidence,
        durable: bool,
    ) -> Result<bool, RoutineError> {
        match (self.is_empty(), evidence) {
            (true, CleanupEvidence::Succeeded | CleanupEvidence::NotRequired) => Ok(false),
            (false, CleanupEvidence::Error(_) | CleanupEvidence::Panic(_)) if durable => Ok(true),
            _ => Err(mediator_error(
                "mediator-reservation-failure-custody-binding-invalid",
            )),
        }
    }
}
