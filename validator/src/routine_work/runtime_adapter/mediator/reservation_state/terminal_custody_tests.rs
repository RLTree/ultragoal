use super::super::terminal_settlement_fixture::*;
use super::super::*;
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::Ordering;

#[test]
fn terminal_settlement_refuses_live_staged_custody() {
    for complete in [true, false] {
        let label = if complete {
            "staged-complete-refusal"
        } else {
            "staged-incomplete-refusal"
        };
        let (attempt, durable, stage_root) = staged_attempt(label);
        let protocol = attempt.protocol_id().clone();
        let error = if complete {
            attempt.settle_success(&BTreeMap::new())
        } else {
            attempt
                .settle_incomplete(DurableSettlement::Failed)
                .map(|_| ())
        }
        .unwrap_err();
        assert_eq!(
            error.cause(),
            "mediator-staged-custody-terminal-transition-refused"
        );
        assert!(durable_settlements(&durable).is_empty());
        assert_eq!(durable.cleanup_calls.load(Ordering::SeqCst), 0);
        assert_eq!(active_grant(&protocol), Some(attempt.grant_id().clone()));

        let result = run_reserved(attempt, |_| {
            Err::<(), _>(mediator_error("post-refusal-cleanup"))
        });
        assert_eq!(result.unwrap_err().cause(), "post-refusal-cleanup");
        assert_eq!(durable.cleanup_calls.load(Ordering::SeqCst), 1);
        assert_eq!(active_grant(&protocol), None);
        assert!(stage_root.is_dir());
        clear_state(&protocol);
        fs::remove_dir_all(stage_root).unwrap();
    }
}

#[test]
fn settled_then_error_and_panic_cannot_bypass_cleanup() {
    for panics in [false, true] {
        let label = if panics {
            "settled-then-panic"
        } else {
            "settled-then-error"
        };
        let (attempt, durable, stage_root) = staged_attempt(label);
        let protocol = attempt.protocol_id().clone();
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            run_reserved(attempt, |attempt| {
                let refusal = attempt
                    .settle_incomplete(DurableSettlement::Failed)
                    .unwrap_err();
                assert_eq!(
                    refusal.cause(),
                    "mediator-staged-custody-terminal-transition-refused"
                );
                if panics {
                    panic!("post-settlement-panic");
                }
                Err::<(), _>(mediator_error("post-settlement-error"))
            })
        }));
        if panics {
            let payload = outcome
                .expect_err("post-settlement panic was replaced")
                .downcast::<&'static str>()
                .expect("post-settlement panic payload changed");
            assert_eq!(*payload, "post-settlement-panic");
        } else {
            assert_eq!(
                outcome.unwrap().unwrap_err().cause(),
                "post-settlement-error"
            );
        }
        assert!(durable_settlements(&durable).is_empty());
        assert_eq!(durable.cleanup_calls.load(Ordering::SeqCst), 1);
        let records = durable
            .failure_records
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].staged_cleanup, CleanupEvidence::Succeeded);
        assert!(matches!(
            (&records[0].primary, panics),
            (FailureEvidence::Panic(_), true) | (FailureEvidence::Error(_), false)
        ));
        drop(records);
        assert_eq!(active_grant(&protocol), None);
        assert!(stage_root.is_dir());
        clear_state(&protocol);
        fs::remove_dir_all(stage_root).unwrap();
    }
}

#[test]
fn terminal_attempt_rejects_new_staged_custody() {
    let durable = Arc::new(TerminalDurable::default());
    let attempt = attempt("stage-after-terminal", Some(durable.clone()), true, None);
    attempt
        .settle_incomplete(DurableSettlement::Failed)
        .unwrap();
    assert!(attempt.terminal_is_authoritative());

    let (stage_root, staged) = staged_fixture("stage-after-terminal-retain");
    let error = attempt
        .stage_and_use(&staged.executable, |_| Ok(()))
        .unwrap_err();
    assert_eq!(
        error.cause(),
        "mediator-reservation-terminal-already-settled"
    );
    assert!(attempt.terminal_is_authoritative());
    assert_eq!(durable.cleanup_calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        durable_settlements(&durable),
        vec![DurableSettlement::Failed]
    );
    fs::remove_dir_all(stage_root).unwrap();
}

fn staged_attempt(label: &str) -> (AttemptReservation, Arc<TerminalDurable>, std::path::PathBuf) {
    let durable = Arc::new(TerminalDurable::default());
    let attempt = attempt(label, Some(durable.clone()), true, None);
    let (stage_root, staged) = staged_fixture(label);
    retain_stage(&attempt, durable.as_ref(), staged);
    (attempt, durable, stage_root)
}

fn durable_settlements(durable: &TerminalDurable) -> Vec<DurableSettlement> {
    durable
        .settlements
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

fn active_grant(protocol: &str) -> Option<String> {
    registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .active_protocols
        .get(protocol)
        .cloned()
}

fn clear_state(protocol: &str) {
    let mut state = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    state.active_protocols.remove(protocol);
    state.ambiguous_protocols.remove(protocol);
    state
        .failure_records
        .retain(|_, record| record.protocol_id != protocol);
}
