use super::producer_state::*;
use super::*;
use std::fs;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::Ordering;

#[test]
fn caller_held_evidence_cannot_skip_cleanup_or_release_authority() {
    let (attempt, durable, stage_root) =
        producer_attempt("caller-held-evidence", CleanupCase::Success);
    let protocol = attempt.protocol_id.clone();
    let grant = attempt.grant_id.clone();
    let marker = attempt.recovery_marker.clone();
    let forged = attempt.failure_evidence(
        FailureEvidence::Error(mediator_error("forged-primary").evidence()),
        CleanupEvidence::NotRequired,
        CleanupEvidence::Succeeded,
    );
    assert_eq!(forged.protocol_id, protocol);
    assert_eq!(forged.grant_id, grant);
    assert_eq!(forged.recovery_marker, marker);

    let error = run_reserved(attempt, |_| {
        Err::<(), _>(mediator_error("crafted-general-error"))
    })
    .unwrap_err();

    assert_eq!(error.cause(), "crafted-general-error");
    let record = recorded(&durable);
    assert_error(&record.primary, "crafted-general-error");
    assert_ne!(record, forged);
    assert_eq!(record.staged_cleanup, CleanupEvidence::Succeeded);
    assert_eq!(durable.cleanup_calls.load(Ordering::SeqCst), 1);
    assert!(!registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .active_protocols
        .contains_key(&protocol));
    clear(&protocol);
    fs::remove_dir_all(stage_root).unwrap();
}

#[test]
fn caller_constructed_transition_error_cannot_skip_cleanup_observation() {
    let (attempt, durable, stage_root) =
        producer_attempt("caller-transition-error", CleanupCase::Success);
    let protocol = attempt.protocol_id.clone();
    let forged = attempt.failure_evidence(
        FailureEvidence::Error(mediator_error("forged-primary").evidence()),
        CleanupEvidence::NotRequired,
        CleanupEvidence::Succeeded,
    );
    let crafted = transition_failure_error(
        "crafted-transition-error",
        forged,
        FailureEvidence::Error(mediator_error("forged-transition").evidence()),
    );

    let error = run_reserved(attempt, |_| Err::<(), _>(crafted)).unwrap_err();

    assert_eq!(error.cause(), "crafted-transition-error");
    let record = recorded(&durable);
    assert_error(&record.primary, "crafted-transition-error");
    assert_eq!(record.staged_cleanup, CleanupEvidence::Succeeded);
    assert_eq!(durable.cleanup_calls.load(Ordering::SeqCst), 1);
    assert!(!registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .active_protocols
        .contains_key(&protocol));
    clear(&protocol);
    fs::remove_dir_all(stage_root).unwrap();
}

#[test]
fn producer_panic_record_failure_returns_transition_and_retains_authority() {
    for cleanup in [CleanupCase::Error, CleanupCase::Panic] {
        let (attempt, durable, stage_root) =
            producer_attempt("producer-panic-record-failure", cleanup);
        *durable
            .failure_record_error
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some("producer-record-error");
        let protocol = attempt.protocol_id.clone();
        let grant = attempt.grant_id.clone();
        let token = mediated_token(usize::MAX, usize::MAX);

        let outcome = catch_unwind(AssertUnwindSafe(|| {
            run_reserved(attempt, |attempt| {
                complete_intent_transition(
                    &token,
                    attempt,
                    incomplete_node(
                        &token,
                        RoutineNodeDisposition::Failed,
                        "MEDIATOR-INCOMPLETE",
                    ),
                )
            })
        }));
        let error = outcome
            .expect("record failure resumed the initiating panic")
            .unwrap_err();
        let transition = error.transition_failure().unwrap();
        assert!(matches!(
            transition.attempted.primary,
            FailureEvidence::Panic(_)
        ));
        assert_eq!(
            transition.attempted.staged_cleanup,
            expected_cleanup(cleanup)
        );
        assert_eq!(durable.cleanup_calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            registry()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .active_protocols
                .get(&protocol),
            Some(&grant)
        );
        clear(&protocol);
        fs::remove_dir_all(stage_root).unwrap();
    }
}
