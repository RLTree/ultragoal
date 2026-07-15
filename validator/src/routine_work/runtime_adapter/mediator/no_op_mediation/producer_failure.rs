use super::producer_state::*;
use super::*;
use std::fs;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::Ordering;

#[derive(Clone, Copy)]
enum ProducerRoute {
    Cancelled,
    DependencyFailed,
    Reused,
    Executed,
    Incomplete,
    MediateError,
}

impl ProducerRoute {
    const ALL: [Self; 6] = [
        Self::Cancelled,
        Self::DependencyFailed,
        Self::Reused,
        Self::Executed,
        Self::Incomplete,
        Self::MediateError,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Cancelled => "cancelled",
            Self::DependencyFailed => "dependency-failed",
            Self::Reused => "reused",
            Self::Executed => "executed",
            Self::Incomplete => "incomplete",
            Self::MediateError => "mediate-error",
        }
    }

    fn node(self, token: &RoutineMediatedIntent) -> RoutineNodeMediation {
        match self {
            Self::Reused => success_node(token, RoutineNodeDisposition::Reused, "0".repeat(64)),
            Self::Executed => success_node(token, RoutineNodeDisposition::Executed, "1".repeat(64)),
            Self::Cancelled => incomplete_node(
                token,
                RoutineNodeDisposition::Cancelled,
                "MEDIATOR-CANCELLED",
            ),
            Self::DependencyFailed => incomplete_node(
                token,
                RoutineNodeDisposition::DependencyFailed,
                "MEDIATOR-DEPENDENCY-FAILED",
            ),
            Self::Incomplete => {
                incomplete_node(token, RoutineNodeDisposition::Failed, "MEDIATOR-INCOMPLETE")
            }
            Self::MediateError => incomplete_node(
                token,
                RoutineNodeDisposition::Failed,
                "MEDIATOR-OBSERVATION-FAILED",
            ),
        }
    }
}

#[test]
fn advance_error_remains_primary_across_cleanup_error_and_panic() {
    for route in ProducerRoute::ALL {
        for cleanup in [CleanupCase::Error, CleanupCase::Panic] {
            let label = format!("producer-{}-advance-error", route.label());
            let (attempt, durable, stage_root) = producer_attempt(&label, cleanup);
            let protocol = attempt.protocol_id.clone();
            let token = mediated_token(1, 0);
            let error = run_reserved(attempt, |attempt| {
                complete_intent_transition(&token, attempt, route.node(&token)).map(drop)
            })
            .unwrap_err();

            assert_eq!(error.cause(), "adapter-intent-transition-order-invalid");
            let record = recorded(&durable);
            assert_error(&record.primary, "adapter-intent-transition-order-invalid");
            assert_eq!(record.staged_cleanup, expected_cleanup(cleanup));
            assert_eq!(durable.cleanup_calls.load(Ordering::SeqCst), 1);
            clear(&protocol);
            fs::remove_dir_all(stage_root).unwrap();
        }
    }
}

#[test]
fn advance_panic_remains_primary_across_cleanup_error_and_panic() {
    for (ordinal, cleanup) in [CleanupCase::Error, CleanupCase::Panic]
        .into_iter()
        .enumerate()
    {
        let label = format!("producer-advance-panic-{ordinal}");
        let (attempt, durable, stage_root) = producer_attempt(&label, cleanup);
        let protocol = attempt.protocol_id.clone();
        let token = mediated_token(usize::MAX, usize::MAX);
        let payload = catch_unwind(AssertUnwindSafe(|| {
            let _: Result<(), RoutineError> = run_reserved(attempt, |attempt| {
                complete_intent_transition(&token, attempt, ProducerRoute::Executed.node(&token))
                    .map(drop)
            });
        }))
        .expect_err("overflowing intent transition did not panic");

        let record = recorded(&durable);
        assert_eq!(
            record.primary,
            FailureEvidence::Panic(PanicEvidence::capture(payload.as_ref()))
        );
        assert_eq!(record.staged_cleanup, expected_cleanup(cleanup));
        assert_eq!(durable.cleanup_calls.load(Ordering::SeqCst), 1);
        clear(&protocol);
        fs::remove_dir_all(stage_root).unwrap();
    }
}

#[test]
fn single_and_terminal_cleanup_failures_keep_the_first_observation() {
    let (attempt, durable, stage_root) =
        producer_attempt("producer-single-advance", CleanupCase::Success);
    let protocol = attempt.protocol_id.clone();
    let token = mediated_token(1, 0);
    let error = run_reserved(attempt, |attempt| {
        complete_intent_transition(&token, attempt, ProducerRoute::Executed.node(&token)).map(drop)
    })
    .unwrap_err();
    assert_eq!(error.cause(), "adapter-intent-transition-order-invalid");
    assert_eq!(
        recorded(&durable).staged_cleanup,
        CleanupEvidence::Succeeded
    );
    assert_eq!(durable.cleanup_calls.load(Ordering::SeqCst), 1);
    clear(&protocol);
    fs::remove_dir_all(stage_root).unwrap();

    let (attempt, durable, stage_root) = producer_attempt("producer-terminal", CleanupCase::Error);
    let protocol = attempt.protocol_id.clone();
    let error = run_reserved(attempt, |attempt| {
        observe_staged_transition(attempt, || Ok(()))
    })
    .unwrap_err();
    assert_eq!(error.cause(), "producer-staged-cleanup-error");
    let record = recorded(&durable);
    assert_error(&record.primary, "producer-staged-cleanup-error");
    assert_eq!(record.staged_cleanup, expected_cleanup(CleanupCase::Error));
    assert_eq!(durable.cleanup_calls.load(Ordering::SeqCst), 1);
    clear(&protocol);
    fs::remove_dir_all(stage_root).unwrap();

    let (attempt, durable, stage_root) =
        producer_attempt("producer-terminal-panic", CleanupCase::Panic);
    let protocol = attempt.protocol_id.clone();
    let payload = catch_unwind(AssertUnwindSafe(|| {
        let _: Result<(), RoutineError> = run_reserved(attempt, |attempt| {
            observe_staged_transition(attempt, || Ok(()))
        });
    }))
    .expect_err("terminal staged cleanup panic was not propagated");
    assert_eq!(
        recorded(&durable).staged_cleanup,
        CleanupEvidence::Panic(PanicEvidence::capture(payload.as_ref()))
    );
    assert_eq!(durable.cleanup_calls.load(Ordering::SeqCst), 1);
    clear(&protocol);
    fs::remove_dir_all(stage_root).unwrap();
}

#[test]
fn producer_transition_failure_retains_exact_active_authority() {
    let (attempt, durable, stage_root) =
        producer_attempt("producer-record-failure", CleanupCase::Error);
    *durable
        .failure_record_error
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some("producer-record-error");
    let protocol = attempt.protocol_id.clone();
    let grant = attempt.grant_id.clone();
    let token = mediated_token(1, 0);
    let error = run_reserved(attempt, |attempt| {
        complete_intent_transition(&token, attempt, ProducerRoute::Executed.node(&token)).map(drop)
    })
    .unwrap_err();

    let transition = error.transition_failure().unwrap();
    assert_error(
        &transition.attempted.primary,
        "adapter-intent-transition-order-invalid",
    );
    assert_eq!(
        transition.attempted.staged_cleanup,
        expected_cleanup(CleanupCase::Error)
    );
    assert_eq!(durable.cleanup_calls.load(Ordering::SeqCst), 1);
    let state = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(state.active_protocols.get(&protocol), Some(&grant));
    drop(state);
    clear(&protocol);
    fs::remove_dir_all(stage_root).unwrap();
}
