use super::terminal_settlement_fixture::*;
use super::*;

#[test]
fn durable_terminal_siblings_clear_exact_ambiguity_and_public_recovery() {
    for (label, outcome, status) in [
        (
            "failed",
            DurableSettlement::Failed,
            RoutineMediatorStatus::IncompleteExecution,
        ),
        (
            "cancelled",
            DurableSettlement::Cancelled,
            RoutineMediatorStatus::Cancelled,
        ),
        (
            "incomplete",
            DurableSettlement::Incomplete,
            RoutineMediatorStatus::IncompleteExecution,
        ),
    ] {
        let durable = Arc::new(TerminalDurable::default());
        let grant = attempt_grant(label, Some(durable.clone()), None);
        let protocol = grant.protocol_id.clone();
        let (_, marker) = run_reserved(&grant, |attempt| {
            attempt.mark_started()?;
            Ok(((), ReservationTerminal::Incomplete(outcome)))
        })
        .unwrap();
        assert_eq!(marker, None);
        let observed = observe_reservation(&protocol, None, None);
        assert_eq!(observed.active_grant, None);
        assert_eq!(observed.recovery_marker, None);
        assert_eq!(durable.settlements.lock().unwrap().as_slice(), &[outcome]);

        let retry = retry_grant(&protocol, durable);
        run_reserved(&retry, |_| {
            Ok((
                (),
                ReservationTerminal::Incomplete(DurableSettlement::Failed),
            ))
        })
        .unwrap();
        let result = RoutineMediationResult {
            request_id: None,
            protocol_id: None,
            status,
            nodes: Vec::new(),
            recovery_marker: marker,
            support_limit: MEDIATOR_SUPPORT_LIMIT,
        };
        assert!(!result.recovery_required());
    }
}

#[test]
fn non_durable_pre_start_terminal_emits_no_marker() {
    let grant = attempt_grant("pre-start-terminal", None, None);
    let protocol = grant.protocol_id.clone();
    let (_, marker) = run_reserved(&grant, |_| {
        Ok((
            (),
            ReservationTerminal::Incomplete(DurableSettlement::Incomplete),
        ))
    })
    .unwrap();
    assert_eq!(marker, None);
    assert_eq!(
        observe_reservation(&protocol, None, None).recovery_marker,
        None
    );
}

#[test]
fn non_durable_recovery_pre_start_retains_prior_marker() {
    let (protocol, prior) = pending_recovery("pre-start-recovery-prior");
    let grant = recovering_grant("pre-start-recovery", &protocol, prior.clone(), None);
    let (_, marker) = run_reserved(&grant, |_| {
        Ok((
            (),
            ReservationTerminal::Incomplete(DurableSettlement::Incomplete),
        ))
    })
    .unwrap();
    assert_eq!(marker, Some(prior.clone()));
    assert_eq!(
        observe_reservation(&protocol, None, None).recovery_marker,
        Some(prior.clone())
    );
    finish_recovery(&protocol, prior);
}

#[test]
fn non_durable_ambiguity_retains_the_exact_pending_marker() {
    let grant = attempt_grant("non-durable", None, None);
    let protocol = grant.protocol_id.clone();
    let expected = Some(grant_recovery_marker(&grant));
    let (_, marker) = run_reserved(&grant, |attempt| {
        attempt.mark_started()?;
        Ok((
            (),
            ReservationTerminal::Incomplete(DurableSettlement::Incomplete),
        ))
    })
    .unwrap();
    assert_eq!(marker, expected);
    assert_eq!(
        observe_reservation(&protocol, None, None).recovery_marker,
        expected
    );
    finish_recovery(&protocol, marker.unwrap());
}

#[test]
fn terminal_cleanup_preserves_foreign_protocol_marker_and_grant() {
    let foreign = attempt_grant("foreign-cleanup-other", None, None);
    let foreign_protocol = foreign.protocol_id.clone();
    let foreign_grant = foreign.grant_id.clone();
    let expected_foreign_marker = grant_recovery_marker(&foreign);
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let (finish_tx, finish_rx) = std::sync::mpsc::channel();
    let foreign_thread = std::thread::spawn(move || {
        run_reserved(&foreign, |attempt| {
            attempt.mark_started()?;
            ready_tx.send(expected_foreign_marker).unwrap();
            finish_rx.recv().unwrap();
            Ok((
                (),
                ReservationTerminal::Incomplete(DurableSettlement::Incomplete),
            ))
        })
    });
    let foreign_marker = ready_rx.recv().unwrap();

    let durable = Arc::new(TerminalDurable::default());
    let grant = attempt_grant("foreign-cleanup", Some(durable), None);
    run_reserved(&grant, |attempt| {
        attempt.mark_started()?;
        Ok((
            (),
            ReservationTerminal::Incomplete(DurableSettlement::Failed),
        ))
    })
    .unwrap();
    let observed = observe_reservation(&foreign_protocol, None, None);
    assert_eq!(observed.active_grant, Some(foreign_grant));
    assert_eq!(observed.recovery_marker, Some(foreign_marker.clone()));

    finish_tx.send(()).unwrap();
    foreign_thread.join().unwrap().unwrap();
    finish_recovery(&foreign_protocol, foreign_marker);
}
