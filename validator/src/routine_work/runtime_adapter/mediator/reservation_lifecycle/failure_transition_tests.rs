use super::super::terminal_settlement_fixture::*;
use super::super::*;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[test]
fn failure_record_error_or_panic_retains_exact_active_authority() {
    for record_panics in [false, true] {
        let label = if record_panics {
            "matrix-record-panic"
        } else {
            "matrix-record-error"
        };
        let durable = Arc::new(TerminalDurable::default());
        let reservation = attempt(label, Some(durable.clone()), true, None);
        let protocol = reservation.protocol_id.clone();
        let grant = reservation.grant_id.clone();
        seed(&reservation, &grant, &reservation.recovery_marker);
        if record_panics {
            *durable
                .failure_record_panic
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some("record-panic");
        } else {
            *durable
                .failure_record_error
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some("record-error");
        }
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            run_reserved(reservation, |_| {
                Err::<(), _>(mediator_error("matrix-primary-error"))
            })
        }));
        let error = outcome.unwrap().unwrap_err();
        let transition = error
            .transition_failure()
            .expect("transition failure was not typed");
        assert_eq!(
            matches!(transition.transition_failure, FailureEvidence::Panic(_)),
            record_panics
        );
        assert_eq!(
            error.cause(),
            if record_panics {
                "mediator-reservation-failure-transition-panicked"
            } else {
                "mediator-reservation-failure-transition-failed"
            }
        );
        let state = registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert_eq!(state.active_protocols.get(&protocol), Some(&grant));
        assert!(
            durable
                .failure_records
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .is_empty()
        );
        drop(state);
        let mut state = registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.active_protocols.remove(&protocol);
        state.ambiguous_protocols.remove(&protocol);
    }
}

#[test]
fn initiating_panic_is_retained_inside_typed_transition_failure() {
    let durable = Arc::new(TerminalDurable::default());
    *durable
        .failure_record_error
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some("record-error");
    let reservation = attempt("panic-record-error", Some(durable), true, None);
    let protocol = reservation.protocol_id.clone();
    let grant = reservation.grant_id.clone();
    seed(&reservation, &grant, &reservation.recovery_marker);
    let failure: RoutineError = run_reserved(reservation, |_| -> Result<(), RoutineError> {
        panic!("initiating-reservation-panic")
    })
    .unwrap_err();
    let transition = failure
        .transition_failure()
        .expect("panic transition failure was not typed");
    assert_eq!(
        transition.attempted.primary,
        FailureEvidence::Panic(PanicEvidence::capture(&"initiating-reservation-panic"))
    );
    let mut state = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(state.active_protocols.get(&protocol), Some(&grant));
    state.active_protocols.remove(&protocol);
    state.ambiguous_protocols.remove(&protocol);
}
