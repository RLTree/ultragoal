use super::super::terminal_settlement_fixture::{TerminalDurable, attempt, seed};
use super::super::{registry, run_reserved};
use super::process_termination_tests::ProcessFixture;
use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

#[test]
fn started_refusal_and_cleanup_failure_are_both_typed() {
    let fixture = ProcessFixture::new("started-refusal");
    let result = fixture.run_result(
        "while :; do :; done",
        Duration::from_secs(2),
        1024,
        &RoutineCancellation::new(),
        || Err(mediator_error("mediator-started-transition-injected")),
    );
    let error = process_error(result);
    assert_eq!(error.cause(), "mediator-started-transition-injected");
    assert!(matches!(
        error.process_custody(),
        Some(ProcessCustodyEvidence {
            primary: FailureEvidence::Error(_),
            cleanup: CleanupEvidence::Succeeded,
        })
    ));
    assert_last_group_absent();
    fixture.teardown();

    let fixture = ProcessFixture::new("cleanup-refusal");
    set_test_process_failure(ProcessFailurePoint::Cleanup, || {});
    let result = fixture.run_result(
        "while :; do :; done",
        Duration::from_secs(2),
        1024,
        &RoutineCancellation::new(),
        || Err(mediator_error("mediator-started-transition-injected")),
    );
    let error = process_error(result);
    assert_eq!(error.cause(), "mediator-process-cleanup-injected");
    assert!(matches!(
        error.process_custody(),
        Some(ProcessCustodyEvidence {
            primary: FailureEvidence::Error(_),
            cleanup: CleanupEvidence::Error(_),
        })
    ));
    assert_last_group_absent();
    fixture.teardown();
}

#[test]
fn running_panic_preserves_payload_and_typed_cleanup() {
    let fixture = ProcessFixture::new("panic");
    set_test_process_failure(ProcessFailurePoint::Wait, || {
        std::panic::panic_any("process-custody-panic")
    });
    let panic = catch_process_panic(&fixture);
    let (panic, evidence) = take_process_custody_panic(panic)
        .unwrap_or_else(|_| panic!("successful cleanup lacked typed custody evidence"));
    assert_eq!(panic.downcast_ref::<&str>(), Some(&"process-custody-panic"));
    assert!(matches!(
        evidence,
        ProcessCustodyEvidence {
            primary: FailureEvidence::Panic(_),
            cleanup: CleanupEvidence::Succeeded,
        }
    ));
    assert_last_group_absent();
    fixture.teardown();
}

#[test]
fn panic_cleanup_error_and_panic_remain_typed() {
    for cleanup_panics in [false, true] {
        let fixture = ProcessFixture::new(if cleanup_panics {
            "panic-cleanup-panic"
        } else {
            "panic-cleanup-error"
        });
        set_test_process_failure(ProcessFailurePoint::Wait, || {
            std::panic::panic_any("process-custody-original-panic")
        });
        append_test_process_failure(ProcessFailurePoint::Cleanup, move || {
            if cleanup_panics {
                std::panic::panic_any("process-custody-cleanup-panic");
            }
        });
        let payload = catch_process_panic(&fixture);
        let (original, evidence) = take_process_custody_panic(payload)
            .unwrap_or_else(|_| panic!("cleanup failure lacked typed custody"));
        assert_eq!(
            original.downcast_ref::<&str>(),
            Some(&"process-custody-original-panic")
        );
        assert_cleanup_evidence(cleanup_panics, evidence);
        assert_last_group_absent();
        fixture.teardown();
    }
}

#[test]
fn real_process_cleanup_evidence_reaches_reservation_authority_before_release() {
    let fixture = ProcessFixture::new("reservation-record");
    let durable = Arc::new(TerminalDurable::default());
    let reservation = attempt("real-process-record", Some(durable.clone()), false, None);
    let protocol = reservation.protocol_id.clone();
    let marker = reservation.recovery_marker.clone();
    seed(&reservation, &reservation.grant_id, &marker);
    set_test_process_failure(ProcessFailurePoint::Cleanup, || {});
    let result = run_reserved(reservation, |attempt| {
        fixture.run_result(
            "while :; do :; done",
            Duration::from_secs(2),
            1024,
            &RoutineCancellation::new(),
            || {
                attempt.mark_started()?;
                Err(mediator_error("mediator-started-transition-injected"))
            },
        )
    });
    let error = process_error(result);
    assert_eq!(error.cause(), "mediator-process-cleanup-injected");
    let records = durable
        .failure_records
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(records.len(), 1);
    assert!(matches!(records[0].primary, FailureEvidence::Error(_)));
    assert!(matches!(
        records[0].process_cleanup,
        CleanupEvidence::Error(_)
    ));
    assert_eq!(records[0].staged_cleanup, CleanupEvidence::Succeeded);
    drop(records);
    let mut state = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert!(!state.active_protocols.contains_key(&protocol));
    assert_eq!(state.ambiguous_protocols.get(&protocol), Some(&marker));
    state.ambiguous_protocols.remove(&protocol);
    drop(state);
    assert_last_group_absent();
    fixture.teardown();
}

fn catch_process_panic(fixture: &ProcessFixture) -> Box<dyn std::any::Any + Send> {
    match catch_unwind(AssertUnwindSafe(|| {
        fixture.run_result(
            "while :; do :; done",
            Duration::from_secs(2),
            1024,
            &RoutineCancellation::new(),
            || Ok(()),
        )
    })) {
        Err(payload) => payload,
        Ok(_) => panic!("injected process panic did not unwind"),
    }
}

fn assert_cleanup_evidence(cleanup_panics: bool, evidence: ProcessCustodyEvidence) {
    assert!(matches!(evidence.primary, FailureEvidence::Panic(_)));
    match (cleanup_panics, evidence.cleanup) {
        (true, CleanupEvidence::Panic(panic)) => assert_eq!(
            panic,
            PanicEvidence::capture(&"process-custody-cleanup-panic")
        ),
        (false, CleanupEvidence::Error(error)) => {
            assert_eq!(error.cause, "mediator-process-cleanup-injected")
        }
        (_, cleanup) => panic!("unexpected cleanup evidence: {cleanup:?}"),
    }
}

fn process_error(
    result: Result<ProcessObservation, crate::routine_work::RoutineError>,
) -> crate::routine_work::RoutineError {
    match result {
        Err(error) => error,
        Ok(_) => panic!("injected process failure unexpectedly succeeded"),
    }
}

fn assert_last_group_absent() {
    let group = test_last_spawn_group().expect("spawned process group was not observed");
    assert!(!process_group_exists(group).unwrap());
}
