use super::*;

pub(crate) fn mediator_error(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::ObservationFailed, cause, None)
}
