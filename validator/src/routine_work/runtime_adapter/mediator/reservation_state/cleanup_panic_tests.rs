use super::super::terminal_settlement_fixture::*;
use super::super::*;
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::Ordering;

fn set_cleanup_panic(durable: &TerminalDurable, payload: &'static str) {
    *durable.cleanup_panic.lock().unwrap() = Some(payload);
}

fn failure_record(durable: &TerminalDurable) -> ReservationFailureEvidence {
    let records = durable.failure_records.lock().unwrap();
    assert_eq!(records.len(), 1);
    records[0].clone()
}

#[test]
fn lifecycle_panic_precedes_cleanup_panic_after_exact_transition() {
    for (label, started) in [("cleanup-panic-pre", false), ("cleanup-panic-post", true)] {
        let durable = Arc::new(TerminalDurable::default());
        set_cleanup_panic(&durable, "reservation-cleanup-panic");
        let grant = attempt_grant(label, Some(durable.clone()), None);
        let protocol = grant.protocol_id.clone();
        let (stage_root, staged) = staged_fixture(label);
        let marker = started.then(|| grant_recovery_marker(&grant));
        let unwound = catch_unwind(AssertUnwindSafe(|| {
            let _: Result<((), Option<String>), RoutineError> = run_reserved(&grant, |attempt| {
                if started {
                    attempt.mark_started()?;
                }
                retain_stage(attempt, durable.as_ref(), staged);
                panic!("reservation-lifecycle-original-payload");
            });
        }));
        assert_eq!(
            unwound.unwrap_err().downcast_ref::<&'static str>().copied(),
            Some("reservation-lifecycle-original-payload")
        );
        assert_eq!(durable.cleanup_calls.load(Ordering::SeqCst), 1);
        let record = failure_record(&durable);
        assert!(matches!(record.primary, FailureEvidence::Panic(_)));
        assert!(matches!(record.staged_cleanup, CleanupEvidence::Panic(_)));
        let observed = observe_reservation(&protocol, Some(&record.recovery_marker), None);
        assert_eq!(observed.active_grant, None);
        assert_eq!(observed.recovery_marker, marker.clone());
        assert_eq!(observed.failure, None);
        if let Some(marker) = marker {
            finish_recovery(&protocol, marker);
        }
        assert!(stage_root.is_dir());
        fs::remove_dir_all(stage_root).unwrap();
    }
}

#[test]
fn cleanup_panic_precedes_error_and_terminal_refusal() {
    for ordinary_error in [true, false] {
        let label = if ordinary_error {
            "cleanup-panic-error"
        } else {
            "cleanup-panic-terminal"
        };
        let durable = Arc::new(TerminalDurable::default());
        set_cleanup_panic(&durable, "reservation-cleanup-result-panic");
        let grant = attempt_grant(label, Some(durable.clone()), None);
        let (stage_root, staged) = staged_fixture(label);
        let unwound = catch_unwind(AssertUnwindSafe(|| {
            run_reserved(&grant, |attempt| {
                retain_stage(attempt, durable.as_ref(), staged);
                if ordinary_error {
                    Err(mediator_error("ordinary-result-error"))
                } else {
                    Ok((
                        (),
                        ReservationTerminal::Incomplete(DurableSettlement::Failed),
                    ))
                }
            })
        }));
        let payload = unwound.unwrap_err();
        assert_eq!(
            payload.downcast_ref::<String>().map(String::as_str),
            Some("reservation-cleanup-result-panic")
        );
        assert_eq!(durable.cleanup_calls.load(Ordering::SeqCst), 1);
        let record = failure_record(&durable);
        assert!(matches!(record.primary, FailureEvidence::Error(_)));
        assert!(matches!(record.staged_cleanup, CleanupEvidence::Panic(_)));
        fs::remove_dir_all(stage_root).unwrap();
    }
}

#[test]
fn primary_error_precedes_cleanup_error() {
    for ordinary_error in [true, false] {
        let label = if ordinary_error {
            "cleanup-error-primary"
        } else {
            "cleanup-error-terminal"
        };
        let durable = Arc::new(TerminalDurable::default());
        *durable.cleanup_failure.lock().unwrap() = Some("reservation-cleanup-result-error");
        let grant = attempt_grant(label, Some(durable.clone()), None);
        let (stage_root, staged) = staged_fixture(label);
        let error = run_reserved(&grant, |attempt| {
            retain_stage(attempt, durable.as_ref(), staged);
            if ordinary_error {
                Err(mediator_error("ordinary-result-error"))
            } else {
                Ok((
                    (),
                    ReservationTerminal::Incomplete(DurableSettlement::Failed),
                ))
            }
        })
        .unwrap_err();
        assert_eq!(
            error.cause(),
            if ordinary_error {
                "ordinary-result-error"
            } else {
                "mediator-staged-custody-terminal-transition-refused"
            }
        );
        assert_eq!(durable.cleanup_calls.load(Ordering::SeqCst), 1);
        assert!(matches!(
            failure_record(&durable).staged_cleanup,
            CleanupEvidence::Error(_)
        ));
        fs::remove_dir_all(stage_root).unwrap();
    }
}
