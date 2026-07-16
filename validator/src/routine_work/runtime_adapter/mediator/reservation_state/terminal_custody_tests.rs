use super::super::terminal_settlement_fixture::*;
use super::super::*;
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::Ordering;

#[test]
fn terminal_settlement_refuses_live_staged_custody() {
    for complete in [true, false] {
        let label = if complete {
            "staged-complete"
        } else {
            "staged-incomplete"
        };
        let durable = Arc::new(TerminalDurable::default());
        let grant = attempt_grant(label, Some(durable.clone()), None);
        let protocol = grant.protocol_id.clone();
        let (stage_root, staged) = staged_fixture(label);
        let error = run_reserved(&grant, |attempt| {
            attempt.mark_started()?;
            retain_stage(attempt, durable.as_ref(), staged);
            let terminal = if complete {
                ReservationTerminal::Complete(BTreeMap::new())
            } else {
                ReservationTerminal::Incomplete(DurableSettlement::Failed)
            };
            Ok(((), terminal))
        })
        .unwrap_err();
        assert_eq!(
            error.cause(),
            "mediator-staged-custody-terminal-transition-refused"
        );
        assert!(durable.settlements.lock().unwrap().is_empty());
        assert_eq!(durable.cleanup_calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            observe_reservation(&protocol, None, None).active_grant,
            None
        );
        assert!(stage_root.is_dir());
        fs::remove_dir_all(stage_root).unwrap();
    }
}

#[test]
fn staged_error_and_panic_preserve_primary_after_cleanup() {
    for panics in [false, true] {
        let label = if panics {
            "staged-panic"
        } else {
            "staged-error"
        };
        let durable = Arc::new(TerminalDurable::default());
        let grant = attempt_grant(label, Some(durable.clone()), None);
        let protocol = grant.protocol_id.clone();
        let (stage_root, staged) = staged_fixture(label);
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            run_reserved(&grant, |attempt| {
                attempt.mark_started()?;
                retain_stage(attempt, durable.as_ref(), staged);
                if panics {
                    panic!("staged-primary-panic");
                }
                Err::<((), ReservationTerminal), _>(mediator_error("staged-primary-error"))
            })
        }));
        if panics {
            assert_eq!(
                outcome.unwrap_err().downcast_ref::<&'static str>().copied(),
                Some("staged-primary-panic")
            );
        } else {
            assert_eq!(
                outcome.unwrap().unwrap_err().cause(),
                "staged-primary-error"
            );
        }
        assert_eq!(durable.cleanup_calls.load(Ordering::SeqCst), 1);
        let records = durable.failure_records.lock().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].staged_cleanup, CleanupEvidence::Succeeded);
        assert!(matches!(
            (&records[0].primary, panics),
            (FailureEvidence::Panic(_), true) | (FailureEvidence::Error(_), false)
        ));
        assert_eq!(
            observe_reservation(&protocol, None, None).active_grant,
            None
        );
        drop(records);
        fs::remove_dir_all(stage_root).unwrap();
    }
}
