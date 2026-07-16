use super::producer_state::*;
use super::*;
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::Ordering;

#[test]
fn caller_held_evidence_cannot_skip_cleanup_or_release_authority() {
    let setup = producer_attempt("caller-held-evidence", CleanupCase::Success);
    let protocol = setup.grant.protocol_id.clone();
    let forged = forged_evidence(&setup.grant);
    let marker = forged.recovery_marker.clone();
    let error = run_reserved(&setup.grant, |attempt| {
        attempt.mark_started()?;
        retain_stage(attempt, setup.durable.as_ref(), setup.staged);
        Err::<((), ReservationTerminal), _>(mediator_error("crafted-general-error"))
    })
    .unwrap_err();
    assert_eq!(error.cause(), "crafted-general-error");
    let record = recorded(&setup.durable);
    assert_error(&record.primary, "crafted-general-error");
    assert_ne!(record, forged);
    assert_eq!(record.staged_cleanup, CleanupEvidence::Succeeded);
    assert_eq!(setup.durable.cleanup_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        observe_reservation(&protocol, None, None).active_grant,
        None
    );
    finish_recovery(&protocol, marker);
    fs::remove_dir_all(setup.stage_root).unwrap();
}

#[test]
fn caller_constructed_transition_error_cannot_skip_cleanup_observation() {
    let setup = producer_attempt("caller-transition-error", CleanupCase::Success);
    let protocol = setup.grant.protocol_id.clone();
    let forged = forged_evidence(&setup.grant);
    let marker = forged.recovery_marker.clone();
    let crafted = transition_failure_error(
        "crafted-transition-error",
        forged,
        FailureEvidence::Error(mediator_error("forged-transition").evidence()),
    );
    let error = run_reserved(&setup.grant, |attempt| {
        attempt.mark_started()?;
        retain_stage(attempt, setup.durable.as_ref(), setup.staged);
        Err::<((), ReservationTerminal), _>(crafted)
    })
    .unwrap_err();
    assert_eq!(error.cause(), "crafted-transition-error");
    let record = recorded(&setup.durable);
    assert_error(&record.primary, "crafted-transition-error");
    assert_eq!(record.staged_cleanup, CleanupEvidence::Succeeded);
    assert_eq!(setup.durable.cleanup_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        observe_reservation(&protocol, None, None).active_grant,
        None
    );
    finish_recovery(&protocol, marker);
    fs::remove_dir_all(setup.stage_root).unwrap();
}

#[test]
fn producer_panic_record_failure_returns_transition_and_retains_authority() {
    for (ordinal, cleanup) in [CleanupCase::Error, CleanupCase::Panic]
        .into_iter()
        .enumerate()
    {
        let setup = producer_attempt(&format!("producer-panic-record-failure-{ordinal}"), cleanup);
        *setup.durable.failure_record_error.lock().unwrap() = Some("producer-record-error");
        let protocol = setup.grant.protocol_id.clone();
        let grant_id = setup.grant.grant_id.clone();
        let token = mediated_token(usize::MAX, usize::MAX);
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            run_reserved(&setup.grant, |attempt| {
                attempt.mark_started()?;
                retain_stage(attempt, setup.durable.as_ref(), setup.staged);
                complete_intent_transition(
                    &token,
                    attempt,
                    incomplete_node(
                        &token,
                        RoutineNodeDisposition::Failed,
                        "MEDIATOR-INCOMPLETE",
                    ),
                )
                .map(|_| {
                    (
                        (),
                        ReservationTerminal::Incomplete(DurableSettlement::Failed),
                    )
                })
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
        assert_eq!(setup.durable.cleanup_calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            observe_reservation(&protocol, None, None).active_grant,
            Some(grant_id)
        );
        fs::remove_dir_all(setup.stage_root).unwrap();
    }
}

fn forged_evidence(grant: &RoutineRootGrant) -> ReservationFailureEvidence {
    ReservationFailureEvidence {
        schema_version: RESERVATION_FAILURE_SCHEMA.to_owned(),
        protocol_id: grant.protocol_id.clone(),
        grant_id: grant.grant_id.clone(),
        recovery_marker: recovery_identity(&grant.grant_id, &grant.protocol_id, &grant.request_id),
        primary: FailureEvidence::Error(mediator_error("forged-primary").evidence()),
        process_cleanup: CleanupEvidence::NotRequired,
        staged_cleanup: CleanupEvidence::Succeeded,
        disposition: ReservationFailureDisposition::StartedPending,
    }
}
