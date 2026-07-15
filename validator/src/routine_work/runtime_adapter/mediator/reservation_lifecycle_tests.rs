use super::terminal_settlement_fixture::*;
use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind};

fn clear(protocol: &str) {
    let mut state = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    state.active_protocols.remove(protocol);
    state.ambiguous_protocols.remove(protocol);
}

#[test]
fn leaving_scope_alone_never_changes_authority_state() {
    let reservation = attempt("drop-inert", None, true, None);
    let protocol = reservation.protocol_id.clone();
    let grant = reservation.grant_id.clone();
    let marker = reservation.recovery_marker.clone();
    seed(&reservation, &grant, &marker);
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
    let reservation = attempt("pre-start", None, false, None);
    let protocol = reservation.protocol_id.clone();
    let grant = reservation.grant_id.clone();
    seed(&reservation, &grant, "foreign-marker");
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
        Some("foreign-marker")
    );
    drop(state);
    clear(&protocol);
}

#[test]
fn explicit_post_start_failure_preserves_exact_ambiguity() {
    let reservation = attempt("post-start", None, false, None);
    let protocol = reservation.protocol_id.clone();
    let grant = reservation.grant_id.clone();
    let marker = reservation.recovery_marker.clone();
    seed(&reservation, &grant, &marker);
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
fn foreign_grant_and_marker_are_never_erased_by_failure() {
    let reservation = attempt("foreign-transition", None, true, None);
    let protocol = reservation.protocol_id.clone();
    seed(&reservation, "foreign-grant", "foreign-marker");
    run_reserved(reservation, |_| {
        Err::<(), _>(mediator_error("foreign-transition-injected"))
    })
    .unwrap_err();

    let state = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(
        state.active_protocols.get(&protocol).map(String::as_str),
        Some("foreign-grant")
    );
    assert_eq!(
        state.ambiguous_protocols.get(&protocol).map(String::as_str),
        Some("foreign-marker")
    );
    drop(state);
    clear(&protocol);
}

#[test]
fn child_start_refuses_a_foreign_marker_without_overwriting_it() {
    let reservation = attempt("foreign-start", None, false, None);
    let protocol = reservation.protocol_id.clone();
    let grant = reservation.grant_id.clone();
    seed(&reservation, &grant, "foreign-marker");
    let error = run_reserved(reservation, |attempt| {
        attempt.mark_started()?;
        Ok(())
    })
    .unwrap_err();
    assert_eq!(error.cause(), "mediator-recovery-marker-conflict");

    let state = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert!(!state.active_protocols.contains_key(&protocol));
    assert_eq!(
        state.ambiguous_protocols.get(&protocol).map(String::as_str),
        Some("foreign-marker")
    );
    drop(state);
    clear(&protocol);
}

#[test]
fn unwind_uses_the_same_explicit_post_start_transition() {
    let reservation = attempt("unwind", None, false, None);
    let protocol = reservation.protocol_id.clone();
    let grant = reservation.grant_id.clone();
    let marker = reservation.recovery_marker.clone();
    seed(&reservation, &grant, &marker);
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
    let reservation = attempt("missing-terminal", None, false, None);
    let protocol = reservation.protocol_id.clone();
    let grant = reservation.grant_id.clone();
    seed(&reservation, &grant, "prior-marker");
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
        Some("prior-marker")
    );
    drop(state);
    clear(&protocol);
}
