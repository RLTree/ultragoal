use super::terminal_settlement_fixture::*;
use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind};

fn clear(protocol: &str) {
    let mut state = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    state.active_protocols.remove(protocol);
    state.ambiguous_protocols.remove(protocol);
    state
        .failure_records
        .retain(|_, record| record.protocol_id != protocol);
}

#[test]
fn leaving_scope_alone_never_changes_authority_state() {
    let reservation = attempt("drop-inert", None, true, None);
    let protocol = reservation.protocol_id().clone();
    let grant = reservation.grant_id().clone();
    let marker = reservation.recovery_marker().clone();
    drop(reservation);

    let state = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(state.active_protocols.get(&protocol), Some(&grant));
    assert_eq!(state.ambiguous_protocols.get(&protocol), Some(&marker));
    drop(state);
    clear(&protocol);
}

#[test]
fn explicit_pre_start_failure_releases_only_exact_active_grant() {
    let (protocol, prior_marker) = pending_recovery("pre-start-prior");
    let reservation = recovering_attempt("pre-start", &protocol, prior_marker.clone(), None, false);
    let error = run_reserved(reservation, |_| {
        Err::<(), _>(mediator_error("pre-start-injected"))
    })
    .unwrap_err();
    assert_eq!(error.cause(), "pre-start-injected");

    let state = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert!(!state.active_protocols.contains_key(&protocol));
    assert_eq!(
        state.ambiguous_protocols.get(&protocol).map(String::as_str),
        Some(prior_marker.as_str())
    );
    drop(state);
    clear(&protocol);
}

#[test]
fn explicit_post_start_failure_preserves_exact_ambiguity() {
    let reservation = attempt("post-start", None, false, None);
    let protocol = reservation.protocol_id().clone();
    let marker = reservation.recovery_marker().clone();
    run_reserved(reservation, |attempt| {
        attempt.mark_started()?;
        Err::<(), _>(mediator_error("post-start-injected"))
    })
    .unwrap_err();

    let state = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert!(!state.active_protocols.contains_key(&protocol));
    assert_eq!(state.ambiguous_protocols.get(&protocol), Some(&marker));
    drop(state);
    clear(&protocol);
}

#[test]
fn other_protocol_grant_and_marker_are_never_erased_by_failure() {
    let foreign = attempt("foreign-transition-other", None, true, None);
    let foreign_protocol = foreign.protocol_id().clone();
    let foreign_grant = foreign.grant_id().clone();
    let foreign_marker = foreign.recovery_marker().clone();
    let reservation = attempt("foreign-transition", None, true, None);
    let protocol = reservation.protocol_id().clone();
    run_reserved(reservation, |_| {
        Err::<(), _>(mediator_error("foreign-transition-injected"))
    })
    .unwrap_err();

    let state = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(
        state
            .active_protocols
            .get(&foreign_protocol)
            .map(String::as_str),
        Some(foreign_grant.as_str())
    );
    assert_eq!(
        state
            .ambiguous_protocols
            .get(&foreign_protocol)
            .map(String::as_str),
        Some(foreign_marker.as_str())
    );
    drop(state);
    foreign
        .settle_incomplete(DurableSettlement::Incomplete)
        .unwrap();
    clear(&protocol);
    clear(&foreign_protocol);
}

#[test]
fn recovery_start_rotates_the_exact_prior_marker() {
    let (protocol, prior_marker) = pending_recovery("foreign-start-prior");
    let reservation = recovering_attempt(
        "foreign-start",
        &protocol,
        prior_marker.clone(),
        None,
        false,
    );
    let marker = reservation.recovery_marker().clone();
    run_reserved(reservation, |attempt| {
        attempt.mark_started()?;
        Err::<(), _>(mediator_error("recovery-start-injected"))
    })
    .unwrap_err();

    let state = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert!(!state.active_protocols.contains_key(&protocol));
    assert_eq!(
        state.ambiguous_protocols.get(&protocol).map(String::as_str),
        Some(marker.as_str())
    );
    drop(state);
    clear(&protocol);
}

#[test]
fn recovery_reservation_refuses_a_wrong_marker() {
    let (protocol, prior_marker) = pending_recovery("wrong-recovery-prior");
    let durable = Arc::new(TerminalDurable::default());
    let grant = recovery_grant(&protocol, Some("wrong-marker".to_owned()), durable);
    let error = match reserve_grant(&grant) {
        Ok(_) => panic!("wrong recovery marker reserved authority"),
        Err(error) => error,
    };
    assert_eq!(error.cause(), "mediator-recovery-authority-required");
    let state = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(
        state.ambiguous_protocols.get(&protocol),
        Some(&prior_marker)
    );
    assert!(!state.active_protocols.contains_key(&protocol));
    drop(state);
    clear(&protocol);
}

#[test]
fn unwind_uses_the_same_explicit_post_start_transition() {
    let reservation = attempt("unwind", None, false, None);
    let protocol = reservation.protocol_id().clone();
    let marker = reservation.recovery_marker().clone();
    let unwound = catch_unwind(AssertUnwindSafe(|| {
        let _: Result<(), RoutineError> = run_reserved(reservation, |attempt| {
            attempt.mark_started()?;
            panic!("reservation-lifecycle-unwind-injected");
        });
    }));
    let payload = unwound.unwrap_err();
    assert_eq!(
        payload.downcast_ref::<&'static str>().copied(),
        Some("reservation-lifecycle-unwind-injected")
    );

    let state = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert!(!state.active_protocols.contains_key(&protocol));
    assert_eq!(state.ambiguous_protocols.get(&protocol), Some(&marker));
    drop(state);
    clear(&protocol);
}

#[test]
fn success_without_terminal_transition_fails_and_releases_reservation() {
    let (protocol, prior_marker) = pending_recovery("missing-terminal-prior");
    let reservation = recovering_attempt(
        "missing-terminal",
        &protocol,
        prior_marker.clone(),
        None,
        false,
    );
    let error = run_reserved(reservation, |_| Ok(())).unwrap_err();
    assert_eq!(
        error.cause(),
        "mediator-reservation-terminal-transition-missing"
    );
    let state = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert!(!state.active_protocols.contains_key(&protocol));
    assert_eq!(
        state.ambiguous_protocols.get(&protocol).map(String::as_str),
        Some(prior_marker.as_str())
    );
    drop(state);
    clear(&protocol);
}
