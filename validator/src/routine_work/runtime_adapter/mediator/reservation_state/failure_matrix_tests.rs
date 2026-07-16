use super::super::terminal_settlement_fixture::*;
use super::super::*;
use crate::routine_work::ProcessCustodyEvidence;
use crate::routine_work::error::PanicPayloadKind;
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};

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
fn opaque_panic_identity_is_recorded_before_the_exact_payload_resumes() {
    let durable = Arc::new(TerminalDurable::default());
    let grant = attempt_grant("matrix-opaque-panic", Some(durable.clone()), None);
    let protocol = grant.protocol_id.clone();
    let marker = grant_recovery_marker(&grant);
    let payload = catch_unwind(AssertUnwindSafe(|| {
        let _: Result<((), Option<String>), RoutineError> = run_reserved(&grant, |attempt| {
            attempt.mark_started()?;
            std::panic::panic_any(OpaquePanic(7));
        });
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
    assert_released(&protocol, Some(&marker));
    finish_recovery(&protocol, marker);
}

fn exercise_failure(
    ordinal: usize,
    started: bool,
    primary_panics: bool,
    process: CleanupCase,
    staged: CleanupCase,
) {
    let label = format!("matrix-{ordinal}");
    let (grant, durable, stage_root, staged_program) = matrix_attempt(&label, staged);
    let protocol = grant.protocol_id.clone();
    let marker = started.then(|| grant_recovery_marker(&grant));
    let process_evidence = ProcessCustodyEvidence {
        primary: if primary_panics {
            FailureEvidence::Panic(panic_evidence("matrix-primary-panic"))
        } else {
            FailureEvidence::Error(mediator_error("matrix-primary-error").evidence())
        },
        cleanup: expected_process(process),
    };
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        run_reserved(&grant, |attempt| {
            if started {
                attempt.mark_started()?;
            }
            retain_stage(attempt, durable.as_ref(), staged_program);
            if primary_panics || matches!(process, CleanupCase::Panic) {
                let payload: Box<dyn std::any::Any + Send> = if primary_panics {
                    Box::new("matrix-primary-panic")
                } else {
                    Box::new("matrix-process-cleanup-panic")
                };
                process::resume_test_process_custody_panic(payload, process_evidence)
            }
            Err::<((), ReservationTerminal), _>(
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
    assert_released(&protocol, marker.as_deref());
    if let Some(marker) = marker {
        finish_recovery(&protocol, marker);
    }
    fs::remove_dir_all(stage_root).unwrap();
}

fn matrix_attempt(
    label: &str,
    staged: CleanupCase,
) -> (
    RoutineRootGrant,
    Arc<TerminalDurable>,
    std::path::PathBuf,
    StagedProgram,
) {
    let durable = Arc::new(TerminalDurable::default());
    match staged {
        CleanupCase::Success => {}
        CleanupCase::Error => {
            *durable.cleanup_failure.lock().unwrap() = Some("matrix-staged-error")
        }
        CleanupCase::Panic => *durable.cleanup_panic.lock().unwrap() = Some("matrix-staged-panic"),
    }
    let grant = attempt_grant(label, Some(durable.clone()), None);
    let (stage_root, staged_program) = staged_fixture(label);
    (grant, durable, stage_root, staged_program)
}

fn recorded(durable: &TerminalDurable) -> ReservationFailureEvidence {
    let records = durable.failure_records.lock().unwrap();
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
        CleanupCase::Panic => {
            CleanupEvidence::Panic(PanicEvidence::capture(&"matrix-staged-panic".to_owned()))
        }
    }
}

fn panic_evidence(payload: &'static str) -> PanicEvidence {
    PanicEvidence::capture(&payload)
}

fn assert_outcome_observed<T>(outcome: std::thread::Result<Result<T, RoutineError>>) {
    assert!(outcome.is_err() || outcome.is_ok_and(|result| result.is_err()));
}

fn assert_released(protocol: &str, expected_marker: Option<&str>) {
    let observed = observe_reservation(protocol, None, None);
    assert_eq!(observed.active_grant, None);
    assert_eq!(observed.recovery_marker.as_deref(), expected_marker);
}
