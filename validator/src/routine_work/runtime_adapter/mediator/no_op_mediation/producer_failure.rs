use super::producer_state::*;
use super::*;
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
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
            let setup = producer_attempt(
                &format!("producer-{}-advance-error", route.label()),
                cleanup,
            );
            let protocol = setup.grant.protocol_id.clone();
            let token = mediated_token(1, 0);
            let marker = grant_recovery_marker(&setup.grant);
            let error = run_reserved(&setup.grant, |attempt| {
                attempt.mark_started()?;
                retain_stage(attempt, setup.durable.as_ref(), setup.staged);
                complete_intent_transition(&token, attempt, route.node(&token)).map(|_| {
                    (
                        (),
                        ReservationTerminal::Incomplete(DurableSettlement::Failed),
                    )
                })
            })
            .unwrap_err();
            assert_eq!(error.cause(), "adapter-intent-transition-order-invalid");
            let record = recorded(&setup.durable);
            assert_error(&record.primary, "adapter-intent-transition-order-invalid");
            assert_eq!(record.staged_cleanup, expected_cleanup(cleanup));
            assert_eq!(setup.durable.cleanup_calls.load(Ordering::SeqCst), 1);
            finish_recovery(&protocol, marker);
            fs::remove_dir_all(setup.stage_root).unwrap();
        }
    }
}

#[test]
fn advance_panic_remains_primary_across_cleanup_error_and_panic() {
    for (ordinal, cleanup) in [CleanupCase::Error, CleanupCase::Panic]
        .into_iter()
        .enumerate()
    {
        let setup = producer_attempt(&format!("producer-advance-panic-{ordinal}"), cleanup);
        let protocol = setup.grant.protocol_id.clone();
        let token = mediated_token(usize::MAX, usize::MAX);
        let marker = grant_recovery_marker(&setup.grant);
        let payload = catch_unwind(AssertUnwindSafe(|| {
            let _: Result<((), Option<String>), RoutineError> =
                run_reserved(&setup.grant, |attempt| {
                    attempt.mark_started()?;
                    retain_stage(attempt, setup.durable.as_ref(), setup.staged);
                    complete_intent_transition(
                        &token,
                        attempt,
                        ProducerRoute::Executed.node(&token),
                    )
                    .map(|_| {
                        (
                            (),
                            ReservationTerminal::Incomplete(DurableSettlement::Failed),
                        )
                    })
                });
        }))
        .expect_err("overflowing intent transition did not panic");
        let record = recorded(&setup.durable);
        assert_eq!(
            record.primary,
            FailureEvidence::Panic(PanicEvidence::capture(payload.as_ref()))
        );
        assert_eq!(record.staged_cleanup, expected_cleanup(cleanup));
        assert_eq!(setup.durable.cleanup_calls.load(Ordering::SeqCst), 1);
        finish_recovery(&protocol, marker);
        fs::remove_dir_all(setup.stage_root).unwrap();
    }
}

#[test]
fn single_and_terminal_cleanup_failures_keep_the_first_observation() {
    let setup = producer_attempt("producer-single-advance", CleanupCase::Success);
    let token = mediated_token(1, 0);
    let error = run_reserved(&setup.grant, |attempt| {
        attempt.mark_started()?;
        retain_stage(attempt, setup.durable.as_ref(), setup.staged);
        complete_intent_transition(&token, attempt, ProducerRoute::Executed.node(&token)).map(
            |_| {
                (
                    (),
                    ReservationTerminal::Incomplete(DurableSettlement::Failed),
                )
            },
        )
    })
    .unwrap_err();
    assert_eq!(error.cause(), "adapter-intent-transition-order-invalid");
    assert_eq!(
        recorded(&setup.durable).staged_cleanup,
        CleanupEvidence::Succeeded
    );
    assert_eq!(setup.durable.cleanup_calls.load(Ordering::SeqCst), 1);
    fs::remove_dir_all(setup.stage_root).unwrap();

    for cleanup in [CleanupCase::Error, CleanupCase::Panic] {
        let setup = producer_attempt("producer-terminal-cleanup", cleanup);
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            run_reserved(&setup.grant, |attempt| {
                retain_stage(attempt, setup.durable.as_ref(), setup.staged);
                observe_staged_transition(attempt, || Ok(())).map(|_| {
                    (
                        (),
                        ReservationTerminal::Incomplete(DurableSettlement::Failed),
                    )
                })
            })
        }));
        assert!(outcome.is_err() || outcome.is_ok_and(|result| result.is_err()));
        assert_eq!(
            recorded(&setup.durable).staged_cleanup,
            expected_cleanup(cleanup)
        );
        assert_eq!(setup.durable.cleanup_calls.load(Ordering::SeqCst), 1);
        fs::remove_dir_all(setup.stage_root).unwrap();
    }
}

#[test]
fn producer_transition_failure_retains_exact_active_authority() {
    let setup = producer_attempt("producer-record-failure", CleanupCase::Error);
    *setup.durable.failure_record_error.lock().unwrap() = Some("producer-record-error");
    let protocol = setup.grant.protocol_id.clone();
    let grant_id = setup.grant.grant_id.clone();
    let token = mediated_token(1, 0);
    let error = run_reserved(&setup.grant, |attempt| {
        attempt.mark_started()?;
        retain_stage(attempt, setup.durable.as_ref(), setup.staged);
        complete_intent_transition(&token, attempt, ProducerRoute::Executed.node(&token)).map(
            |_| {
                (
                    (),
                    ReservationTerminal::Incomplete(DurableSettlement::Failed),
                )
            },
        )
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
    assert_eq!(setup.durable.cleanup_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        observe_reservation(&protocol, None, None).active_grant,
        Some(grant_id)
    );
    fs::remove_dir_all(setup.stage_root).unwrap();
}
