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
        *durable
            .cleanup_failure
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(cleanup_cause);
        let reservation = attempt(label, Some(durable.clone()), false, None);
        let protocol = reservation.protocol_id.clone();
        let grant = reservation.grant_id.clone();
        let marker = reservation.recovery_marker.clone();
        let foreign_protocol = format!("{protocol}-foreign");
        let (stage_root, staged) = staged_fixture(label);
        reservation.staged.borrow_mut().push(staged);
        {
            let mut state = registry()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state.active_protocols.insert(protocol.clone(), grant);
            state
                .ambiguous_protocols
                .insert(foreign_protocol.clone(), "foreign-marker".to_owned());
        }

        let unwound = catch_unwind(AssertUnwindSafe(|| {
            let _: Result<(), RoutineError> = run_reserved(reservation, |attempt| {
                attempt.mark_started()?;
                panic!("reservation-unwind-original-payload");
            });
        }));
        let payload = unwound.unwrap_err();
        assert_eq!(
            payload.downcast_ref::<&'static str>().copied(),
            Some("reservation-unwind-original-payload")
        );
        assert_eq!(durable.cleanup_calls.load(Ordering::SeqCst), 1);
        assert!(stage_root.is_dir());
        {
            let state = registry()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            assert!(!state.active_protocols.contains_key(&protocol));
            assert_eq!(state.ambiguous_protocols.get(&protocol), Some(&marker));
            assert_eq!(
                state
                    .ambiguous_protocols
                    .get(&foreign_protocol)
                    .map(String::as_str),
                Some("foreign-marker")
            );
        }

        let recovery_durable = Arc::new(TerminalDurable::default());
        let mut recovery = retry_grant(&attempt(label, None, false, None), recovery_durable);
        recovery.protocol_id.clone_from(&protocol);
        recovery.recovery_for = Some(marker);
        let retry = reserve_grant(&recovery).unwrap();
        retry.settle_incomplete(DurableSettlement::Failed).unwrap();
        let mut state = registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert!(!state.active_protocols.contains_key(&protocol));
        assert!(!state.ambiguous_protocols.contains_key(&protocol));
        assert_eq!(
            state
                .ambiguous_protocols
                .get(&foreign_protocol)
                .map(String::as_str),
            Some("foreign-marker")
        );
        state.ambiguous_protocols.remove(&foreign_protocol);
        drop(state);
        fs::remove_dir_all(stage_root).unwrap();
    }
}
