use super::terminal_settlement_fixture::*;
use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[test]
fn explicit_pre_start_failure_releases_only_exact_active_grant() {
    let (protocol, prior) = pending_recovery("pre-start-prior");
    let grant = recovering_grant("pre-start", &protocol, prior.clone(), None);
    let error = run_reserved(&grant, |_| {
        Err::<((), ReservationTerminal), _>(mediator_error("pre-start-injected"))
    })
    .unwrap_err();
    assert_eq!(error.cause(), "pre-start-injected");
    let observed = observe_reservation(&protocol, None, None);
    assert_eq!(observed.active_grant, None);
    assert_eq!(observed.recovery_marker, Some(prior.clone()));
    finish_recovery(&protocol, prior);
}

#[test]
fn explicit_post_start_failure_preserves_exact_ambiguity() {
    let grant = attempt_grant("post-start", None, None);
    let protocol = grant.protocol_id.clone();
    let marker = grant_recovery_marker(&grant);
    run_reserved(&grant, |attempt| {
        attempt.mark_started()?;
        Err::<((), ReservationTerminal), _>(mediator_error("post-start-injected"))
    })
    .unwrap_err();
    let observed = observe_reservation(&protocol, None, None);
    assert_eq!(observed.active_grant, None);
    assert_eq!(observed.recovery_marker, Some(marker.clone()));
    finish_recovery(&protocol, marker);
}

#[test]
fn other_protocol_grant_and_marker_are_never_erased_by_failure() {
    let foreign = attempt_grant("foreign-transition-other", None, None);
    let foreign_protocol = foreign.protocol_id.clone();
    let foreign_grant = foreign.grant_id.clone();
    let foreign_marker = grant_recovery_marker(&foreign);
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let (finish_tx, finish_rx) = std::sync::mpsc::channel();
    let foreign_thread = std::thread::spawn(move || {
        run_reserved(&foreign, |attempt| {
            attempt.mark_started()?;
            ready_tx.send(foreign_marker).unwrap();
            finish_rx.recv().unwrap();
            Ok((
                (),
                ReservationTerminal::Incomplete(DurableSettlement::Incomplete),
            ))
        })
    });
    let foreign_marker = ready_rx.recv().unwrap();

    let grant = attempt_grant("foreign-transition", None, None);
    run_reserved(&grant, |attempt| {
        attempt.mark_started()?;
        Err::<((), ReservationTerminal), _>(mediator_error("foreign-transition-injected"))
    })
    .unwrap_err();
    let observed = observe_reservation(&foreign_protocol, None, None);
    assert_eq!(observed.active_grant, Some(foreign_grant));
    assert_eq!(observed.recovery_marker, Some(foreign_marker.clone()));

    finish_tx.send(()).unwrap();
    foreign_thread.join().unwrap().unwrap();
    finish_recovery(&foreign_protocol, foreign_marker);
}

#[test]
fn recovery_start_rotates_the_exact_prior_marker() {
    let (protocol, prior) = pending_recovery("foreign-start-prior");
    let grant = recovering_grant("foreign-start", &protocol, prior, None);
    let marker = grant_recovery_marker(&grant);
    run_reserved(&grant, |attempt| {
        attempt.mark_started()?;
        Err::<((), ReservationTerminal), _>(mediator_error("recovery-start-injected"))
    })
    .unwrap_err();
    let observed = observe_reservation(&protocol, None, None);
    assert_eq!(observed.active_grant, None);
    assert_eq!(observed.recovery_marker, Some(marker.clone()));
    finish_recovery(&protocol, marker);
}

#[test]
fn recovery_reservation_refuses_a_wrong_marker_before_lifecycle() {
    let (protocol, prior) = pending_recovery("wrong-recovery-prior");
    let durable = Arc::new(TerminalDurable::default());
    let grant = recovery_grant(&protocol, Some("wrong-marker".to_owned()), durable);
    let entered = std::cell::Cell::new(false);
    let error = run_reserved(&grant, |_| {
        entered.set(true);
        Ok((
            (),
            ReservationTerminal::Incomplete(DurableSettlement::Failed),
        ))
    })
    .unwrap_err();
    assert_eq!(error.cause(), "mediator-recovery-authority-required");
    assert!(!entered.get());
    let observed = observe_reservation(&protocol, None, None);
    assert_eq!(observed.active_grant, None);
    assert_eq!(observed.recovery_marker, Some(prior.clone()));
    finish_recovery(&protocol, prior);
}

#[test]
fn unwind_uses_the_same_explicit_post_start_transition() {
    let grant = attempt_grant("unwind", None, None);
    let protocol = grant.protocol_id.clone();
    let marker = grant_recovery_marker(&grant);
    let unwound = catch_unwind(AssertUnwindSafe(|| {
        let _: Result<((), Option<String>), RoutineError> = run_reserved(&grant, |attempt| {
            attempt.mark_started()?;
            panic!("reservation-lifecycle-unwind-injected");
        });
    }));
    assert_eq!(
        unwound.unwrap_err().downcast_ref::<&'static str>().copied(),
        Some("reservation-lifecycle-unwind-injected")
    );
    let observed = observe_reservation(&protocol, None, None);
    assert_eq!(observed.active_grant, None);
    assert_eq!(observed.recovery_marker, Some(marker.clone()));
    finish_recovery(&protocol, marker);
}
