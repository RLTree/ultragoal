use super::super::terminal_settlement_fixture::*;
use super::super::*;
use std::fs;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::Ordering;

fn set_cleanup_panic(durable: &TerminalDurable, payload: &'static str) {
    *durable
        .cleanup_panic
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(payload);
}

fn cleanup_panic(payload: &(dyn std::any::Any + Send)) -> Option<&str> {
    payload.downcast_ref::<String>().map(String::as_str)
}

fn failure_record(durable: &TerminalDurable) -> ReservationFailureEvidence {
    let records = durable
        .failure_records
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(records.len(), 1);
    records[0].clone()
}

fn clear(protocol: &str) {
    let mut state = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    state.active_protocols.remove(protocol);
    state.ambiguous_protocols.remove(protocol);
    state
        .failure_records
        .retain(|_, record| record.protocol_id != protocol);
}

#[test]
fn lifecycle_panic_precedes_cleanup_panic_after_exact_transition() {
    for (label, started) in [("cleanup-panic-pre", false), ("cleanup-panic-post", true)] {
        let durable = Arc::new(TerminalDurable::default());
        set_cleanup_panic(&durable, "reservation-cleanup-panic");
        let reservation = attempt(label, Some(durable.clone()), false, None);
        let protocol = reservation.protocol_id().clone();
        let grant = reservation.grant_id().clone();
        let marker = reservation.recovery_marker().clone();
        let foreign_protocol = format!("{protocol}-foreign");
        let (stage_root, staged) = staged_fixture(label);
        retain_stage(&reservation, durable.as_ref(), staged);
        {
            let mut state = registry()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state.active_protocols.insert(protocol.clone(), grant);
            state
                .ambiguous_protocols
                .insert(foreign_protocol.clone(), "foreign-marker".to_owned());
        }

        let unwound = catch_unwind(AssertUnwindSafe(|| {
            let _: Result<(), RoutineError> = run_reserved(reservation, |attempt| {
                if started {
                    attempt.mark_started()?;
                }
                panic!("reservation-lifecycle-original-payload");
            });
        }));
        let payload = unwound.unwrap_err();
        assert_eq!(
            payload.downcast_ref::<&'static str>().copied(),
            Some("reservation-lifecycle-original-payload")
        );
        assert_eq!(durable.cleanup_calls.load(Ordering::SeqCst), 1);
        let record = failure_record(&durable);
        assert!(matches!(record.primary, FailureEvidence::Panic(_)));
        assert!(matches!(record.staged_cleanup, CleanupEvidence::Panic(_)));
        assert!(stage_root.is_dir());
        {
            let state = registry()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            assert!(!state.active_protocols.contains_key(&protocol));
            assert_eq!(
                state.ambiguous_protocols.get(&protocol),
                started.then_some(&marker)
            );
            assert_eq!(
                state
                    .ambiguous_protocols
                    .get(&foreign_protocol)
                    .map(String::as_str),
                Some("foreign-marker")
            );
        }

        let recovery_durable = Arc::new(TerminalDurable::default());
        let mut recovery = retry_grant(&attempt(label, None, false, None), recovery_durable);
        recovery.protocol_id.clone_from(&protocol);
        recovery.recovery_for = started.then_some(marker);
        let retry = reserve_grant(&recovery).unwrap();
        retry.settle_incomplete(DurableSettlement::Failed).unwrap();
        let mut state = registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert!(!state.active_protocols.contains_key(&protocol));
        assert!(!state.ambiguous_protocols.contains_key(&protocol));
        assert_eq!(
            state
                .ambiguous_protocols
                .get(&foreign_protocol)
                .map(String::as_str),
            Some("foreign-marker")
        );
        state.ambiguous_protocols.remove(&foreign_protocol);
        drop(state);
        fs::remove_dir_all(stage_root).unwrap();
    }
}

#[test]
fn cleanup_panic_precedes_result_outcomes_after_exact_transition() {
    for (label, started, ordinary_error) in [
        ("cleanup-panic-error", false, true),
        ("cleanup-panic-missing", true, false),
    ] {
        let durable = Arc::new(TerminalDurable::default());
        set_cleanup_panic(&durable, "reservation-cleanup-result-panic");
        let reservation = attempt(label, Some(durable.clone()), false, None);
        let protocol = reservation.protocol_id().clone();
        let grant = reservation.grant_id().clone();
        let marker = reservation.recovery_marker().clone();
        let (stage_root, staged) = staged_fixture(label);
        retain_stage(&reservation, durable.as_ref(), staged);
        seed(
            &reservation,
            &grant,
            if started { &marker } else { "foreign-marker" },
        );

        let unwound = catch_unwind(AssertUnwindSafe(|| {
            run_reserved(reservation, |attempt| {
                if started {
                    attempt.mark_started()?;
                }
                if ordinary_error {
                    Err(mediator_error("ordinary-result-error"))
                } else {
                    Ok(())
                }
            })
        }));
        let payload = unwound.unwrap_err();
        assert_eq!(
            cleanup_panic(payload.as_ref()),
            Some("reservation-cleanup-result-panic")
        );
        assert_eq!(durable.cleanup_calls.load(Ordering::SeqCst), 1);
        let record = failure_record(&durable);
        assert_eq!(
            matches!(record.primary, FailureEvidence::Error(_)),
            ordinary_error
        );
        assert!(matches!(record.staged_cleanup, CleanupEvidence::Panic(_)));
        assert!(stage_root.is_dir());
        let state = registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert!(!state.active_protocols.contains_key(&protocol));
        assert_eq!(
            state.ambiguous_protocols.get(&protocol).map(String::as_str),
            Some(if started {
                marker.as_str()
            } else {
                "foreign-marker"
            })
        );
        drop(state);
        clear(&protocol);
        fs::remove_dir_all(stage_root).unwrap();
    }
}

#[test]
fn ordinary_error_precedes_cleanup_error_but_missing_transition_does_not() {
    for (label, ordinary_error, expected) in [
        ("cleanup-error-result", true, "ordinary-result-error"),
        (
            "cleanup-error-missing",
            false,
            "reservation-cleanup-result-error",
        ),
    ] {
        let durable = Arc::new(TerminalDurable::default());
        *durable
            .cleanup_failure
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) =
            Some("reservation-cleanup-result-error");
        let reservation = attempt(label, Some(durable.clone()), false, None);
        let protocol = reservation.protocol_id().clone();
        let grant = reservation.grant_id().clone();
        let (stage_root, staged) = staged_fixture(label);
        retain_stage(&reservation, durable.as_ref(), staged);
        seed(&reservation, &grant, "foreign-marker");

        let error = run_reserved(reservation, |_| {
            if ordinary_error {
                Err(mediator_error("ordinary-result-error"))
            } else {
                Ok(())
            }
        })
        .unwrap_err();
        assert_eq!(error.cause(), expected);
        assert_eq!(durable.cleanup_calls.load(Ordering::SeqCst), 1);
        let record = failure_record(&durable);
        assert_eq!(
            matches!(record.primary, FailureEvidence::Error(_)),
            ordinary_error
        );
        assert!(matches!(record.staged_cleanup, CleanupEvidence::Error(_)));
        assert!(stage_root.is_dir());
        let state = registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert!(!state.active_protocols.contains_key(&protocol));
        assert_eq!(
            state.ambiguous_protocols.get(&protocol).map(String::as_str),
            Some("foreign-marker")
        );
        drop(state);
        clear(&protocol);
        fs::remove_dir_all(stage_root).unwrap();
    }
}
