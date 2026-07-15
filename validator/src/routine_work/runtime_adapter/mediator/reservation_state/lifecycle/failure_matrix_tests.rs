use super::super::terminal_settlement_fixture::*;
use super::super::*;
use crate::routine_work::error::PanicPayloadKind;
use std::fs;
use std::panic::{catch_unwind, AssertUnwindSafe};

#[derive(Clone, Copy, Debug)]
enum CleanupCase {
    Success,
    Error,
    Panic,
}

const CASES: [CleanupCase; 3] = [CleanupCase::Success, CleanupCase::Error, CleanupCase::Panic];

struct OpaquePanic(u8);

#[test]
fn error_and_panic_cleanup_cross_product_is_recorded_before_release() {
    let mut ordinal = 0;
    for started in [false, true] {
        for process in CASES {
            for staged in CASES {
                for primary_panics in [false, true] {
                    ordinal += 1;
                    exercise_failure(ordinal, started, primary_panics, process, staged);
                }
            }
        }
    }
}

#[test]
fn missing_settlement_records_no_process_cleanup_and_every_staged_outcome() {
    for (ordinal, staged) in CASES.into_iter().enumerate() {
        let label = format!("matrix-missing-{ordinal}");
        let (reservation, durable, stage_root) = matrix_attempt(&label, true, staged);
        let protocol = reservation.protocol_id().clone();
        let marker = reservation.recovery_marker().clone();
        let result = catch_unwind(AssertUnwindSafe(|| {
            run_reserved(reservation, |_| Ok::<(), RoutineError>(()))
        }));
        assert_outcome_observed(result);
        let record = recorded(&durable);
        assert_eq!(record.primary, FailureEvidence::MissingSettlement);
        assert_eq!(record.process_cleanup, CleanupEvidence::NotRequired);
        assert_eq!(record.staged_cleanup, expected_staged(staged));
        assert_released(&protocol, Some(marker.as_str()));
        fs::remove_dir_all(stage_root).unwrap();
    }
}

#[test]
fn opaque_panic_identity_is_recorded_before_the_exact_payload_resumes() {
    let durable = Arc::new(TerminalDurable::default());
    let reservation = attempt("matrix-opaque-panic", Some(durable.clone()), true, None);
    let protocol = reservation.protocol_id().clone();
    let marker = reservation.recovery_marker().clone();
    seed(&reservation, reservation.grant_id(), &marker);
    let payload = catch_unwind(AssertUnwindSafe(|| {
        let _: Result<(), RoutineError> =
            run_reserved(reservation, |_| std::panic::panic_any(OpaquePanic(7)));
    }))
    .expect_err("opaque panic was swallowed");
    assert_eq!(
        payload.downcast_ref::<OpaquePanic>().map(|value| value.0),
        Some(7)
    );
    let record = recorded(&durable);
    assert!(matches!(
        record.primary,
        FailureEvidence::Panic(PanicEvidence {
            payload_kind: PanicPayloadKind::OpaqueType,
            payload_byte_length: 0,
            ..
        })
    ));
    assert_released(&protocol, Some(marker.as_str()));
}

fn exercise_failure(
    ordinal: usize,
    started: bool,
    primary_panics: bool,
    process: CleanupCase,
    staged: CleanupCase,
) {
    let label = format!("matrix-{ordinal}");
    let (reservation, durable, stage_root) = matrix_attempt(&label, started, staged);
    let protocol = reservation.protocol_id().clone();
    let marker = reservation.recovery_marker().clone();
    let process_evidence = ProcessCustodyEvidence {
        primary: if primary_panics {
            FailureEvidence::Panic(panic_evidence("matrix-primary-panic"))
        } else {
            FailureEvidence::Error(mediator_error("matrix-primary-error").evidence())
        },
        cleanup: expected_process(process),
    };
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        run_reserved(reservation, |_| {
            if primary_panics || matches!(process, CleanupCase::Panic) {
                let payload: Box<dyn std::any::Any + Send> = if primary_panics {
                    Box::new("matrix-primary-panic")
                } else {
                    Box::new("matrix-process-cleanup-panic")
                };
                process::resume_test_process_custody_panic(payload, process_evidence)
            }
            Err::<(), _>(
                mediator_error("matrix-primary-error").with_process_custody(process_evidence),
            )
        })
    }));
    assert_outcome_observed(outcome);
    let record = recorded(&durable);
    assert_eq!(record.process_cleanup, expected_process(process));
    assert_eq!(record.staged_cleanup, expected_staged(staged));
    assert_eq!(
        record.disposition,
        if started {
            ReservationFailureDisposition::StartedPending
        } else {
            ReservationFailureDisposition::ReservedPending
        }
    );
    let expected_marker = started
        .then_some(marker.as_str())
        .unwrap_or("matrix-foreign-marker");
    assert_released(&protocol, Some(expected_marker));
    fs::remove_dir_all(stage_root).unwrap();
}

fn matrix_attempt(
    label: &str,
    started: bool,
    staged: CleanupCase,
) -> (AttemptReservation, Arc<TerminalDurable>, std::path::PathBuf) {
    let durable = Arc::new(TerminalDurable::default());
    match staged {
        CleanupCase::Success => {}
        CleanupCase::Error => {
            *durable
                .cleanup_failure
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some("matrix-staged-error")
        }
        CleanupCase::Panic => {
            *durable
                .cleanup_panic
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some("matrix-staged-panic")
        }
    }
    let reservation = attempt(label, Some(durable.clone()), started, None);
    let (stage_root, staged_program) = staged_fixture(label);
    retain_stage(&reservation, durable.as_ref(), staged_program);
    let ambiguity = if started {
        reservation.recovery_marker().as_str()
    } else {
        "matrix-foreign-marker"
    };
    seed(&reservation, reservation.grant_id(), ambiguity);
    (reservation, durable, stage_root)
}

fn recorded(durable: &TerminalDurable) -> ReservationFailureEvidence {
    let records = durable
        .failure_records
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(records.len(), 1);
    records[0].clone()
}

fn expected_process(case: CleanupCase) -> CleanupEvidence {
    match case {
        CleanupCase::Success => CleanupEvidence::Succeeded,
        CleanupCase::Error => {
            CleanupEvidence::Error(mediator_error("matrix-process-cleanup-error").evidence())
        }
        CleanupCase::Panic => {
            CleanupEvidence::Panic(panic_evidence("matrix-process-cleanup-panic"))
        }
    }
}

fn expected_staged(case: CleanupCase) -> CleanupEvidence {
    match case {
        CleanupCase::Success => CleanupEvidence::Succeeded,
        CleanupCase::Error => {
            CleanupEvidence::Error(mediator_error("matrix-staged-error").evidence())
        }
        CleanupCase::Panic => CleanupEvidence::Panic(string_panic_evidence("matrix-staged-panic")),
    }
}

fn panic_evidence(payload: &'static str) -> PanicEvidence {
    PanicEvidence::capture(&payload)
}

fn string_panic_evidence(payload: &str) -> PanicEvidence {
    PanicEvidence::capture(&payload.to_owned())
}

fn assert_outcome_observed<T>(outcome: std::thread::Result<Result<T, RoutineError>>) {
    assert!(outcome.is_err() || outcome.is_ok_and(|result| result.is_err()));
}

fn assert_released(protocol: &str, expected_marker: Option<&str>) {
    let state = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert!(!state.active_protocols.contains_key(protocol));
    assert_eq!(
        state.ambiguous_protocols.get(protocol).map(String::as_str),
        expected_marker
    );
    drop(state);
    clear(protocol);
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
