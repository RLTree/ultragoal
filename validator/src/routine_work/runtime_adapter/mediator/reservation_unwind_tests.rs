use super::terminal_settlement_fixture::*;
use super::*;
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::Ordering;

#[test]
fn cleanup_failures_never_replace_the_initiating_panic_or_erase_recovery() {
    for (label, cleanup_cause) in [
        ("directory", "routine-production-launch-directory-mismatch"),
        ("entry", "routine-production-launch-entry-mismatch"),
        ("missing", "routine-production-launch-entry-missing"),
    ] {
        let durable = Arc::new(TerminalDurable::default());
        *durable.cleanup_failure.lock().unwrap() = Some(cleanup_cause);
        let grant = attempt_grant(label, Some(durable.clone()), None);
        let protocol = grant.protocol_id.clone();
        let (stage_root, staged) = staged_fixture(label);
        let marker = grant_recovery_marker(&grant);
        let unwound = catch_unwind(AssertUnwindSafe(|| {
            let _: Result<((), Option<String>), RoutineError> = run_reserved(&grant, |attempt| {
                attempt.mark_started()?;
                retain_stage(attempt, durable.as_ref(), staged);
                panic!("reservation-unwind-original-payload");
            });
        }));
        assert_eq!(
            unwound.unwrap_err().downcast_ref::<&'static str>().copied(),
            Some("reservation-unwind-original-payload")
        );
        assert_eq!(durable.cleanup_calls.load(Ordering::SeqCst), 1);
        let records = durable.failure_records.lock().unwrap();
        assert_eq!(records.len(), 1);
        assert!(matches!(records[0].primary, FailureEvidence::Panic(_)));
        assert!(matches!(
            records[0].staged_cleanup,
            CleanupEvidence::Error(_)
        ));
        drop(records);
        let observed = observe_reservation(&protocol, None, None);
        assert_eq!(observed.active_grant, None);
        assert_eq!(observed.recovery_marker, Some(marker.clone()));
        finish_recovery(&protocol, marker);
        assert!(stage_root.is_dir());
        fs::remove_dir_all(stage_root).unwrap();
    }
}
