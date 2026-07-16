use super::super::terminal_settlement_fixture::*;
use super::super::*;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[test]
fn failure_record_error_or_panic_retains_exact_active_authority() {
    for record_panics in [false, true] {
        let label = if record_panics {
            "record-panic"
        } else {
            "record-error"
        };
        let durable = Arc::new(TerminalDurable::default());
        if record_panics {
            *durable.failure_record_panic.lock().unwrap() = Some("record-panic");
        } else {
            *durable.failure_record_error.lock().unwrap() = Some("record-error");
        }
        let grant = attempt_grant(label, Some(durable.clone()), None);
        let protocol = grant.protocol_id.clone();
        let grant_id = grant.grant_id.clone();
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            run_reserved(&grant, |_| {
                Err::<((), ReservationTerminal), _>(mediator_error("matrix-primary-error"))
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
        assert_eq!(
            observe_reservation(&protocol, None, None).active_grant,
            Some(grant_id)
        );
        assert!(durable.failure_records.lock().unwrap().is_empty());
    }
}

#[test]
fn initiating_panic_is_retained_inside_typed_transition_failure() {
    let durable = Arc::new(TerminalDurable::default());
    *durable.failure_record_error.lock().unwrap() = Some("record-error");
    let grant = attempt_grant("panic-record-error", Some(durable), None);
    let protocol = grant.protocol_id.clone();
    let grant_id = grant.grant_id.clone();
    let failure: RoutineError = run_reserved(
        &grant,
        |_| -> Result<((), ReservationTerminal), RoutineError> {
            panic!("initiating-reservation-panic")
        },
    )
    .unwrap_err();
    let transition = failure
        .transition_failure()
        .expect("panic transition failure was not typed");
    assert_eq!(
        transition.attempted.primary,
        FailureEvidence::Panic(PanicEvidence::capture(&"initiating-reservation-panic"))
    );
    assert_eq!(
        observe_reservation(&protocol, None, None).active_grant,
        Some(grant_id)
    );
}
