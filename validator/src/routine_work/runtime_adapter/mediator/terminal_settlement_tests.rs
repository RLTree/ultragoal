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
        let reservation = attempt(label, Some(durable.clone()), true, None);
        let marker = reservation.settle_incomplete(outcome).unwrap();
        assert_eq!(marker, None);
        let state = registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert!(
            !state
                .active_protocols
                .contains_key(reservation.protocol_id())
        );
        assert!(
            !state
                .ambiguous_protocols
                .contains_key(reservation.protocol_id())
        );
        drop(state);
        assert_eq!(
            *durable
                .settlements
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
            vec![outcome]
        );
        let retry = reserve_grant(&retry_grant(&reservation, durable)).unwrap();
        retry.settle_incomplete(DurableSettlement::Failed).unwrap();
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
    let reservation = attempt("pre-start-terminal", None, false, None);
    assert_eq!(
        reservation
            .settle_incomplete(DurableSettlement::Incomplete)
            .unwrap(),
        None
    );
    let state = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert!(
        !state
            .ambiguous_protocols
            .contains_key(reservation.protocol_id())
    );
}

#[test]
fn non_durable_recovery_pre_start_retains_prior_marker() {
    let (protocol, prior_marker) = pending_recovery("pre-start-recovery-prior");
    let reservation = recovering_attempt(
        "pre-start-recovery",
        &protocol,
        prior_marker.clone(),
        None,
        false,
    );
    assert_eq!(
        reservation
            .settle_incomplete(DurableSettlement::Incomplete)
            .unwrap(),
        Some(prior_marker.clone())
    );
    let state = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(
        state.ambiguous_protocols.get(&protocol),
        Some(&prior_marker)
    );
    drop(state);
    clear_protocol(&protocol);
}

#[test]
fn non_durable_ambiguity_retains_the_exact_pending_marker() {
    let reservation = attempt("non-durable", None, true, None);
    assert_eq!(
        reservation
            .settle_incomplete(DurableSettlement::Incomplete)
            .unwrap(),
        Some(reservation.recovery_marker().clone())
    );
    let mut state = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(
        state.ambiguous_protocols.get(reservation.protocol_id()),
        Some(reservation.recovery_marker())
    );
    state.ambiguous_protocols.remove(reservation.protocol_id());
}

#[test]
fn terminal_cleanup_preserves_foreign_protocol_marker_and_grant() {
    let durable = Arc::new(TerminalDurable::default());
    let reservation = attempt("foreign-cleanup", Some(durable), true, None);
    let foreign = attempt("foreign-cleanup-other", None, true, None);
    let foreign_protocol = foreign.protocol_id().clone();
    let foreign_grant = foreign.grant_id().clone();
    let foreign_marker = foreign.recovery_marker().clone();
    assert_eq!(
        reservation
            .settle_incomplete(DurableSettlement::Failed)
            .unwrap(),
        None
    );
    let state = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(
        state.active_protocols.get(&foreign_protocol),
        Some(&foreign_grant)
    );
    assert_eq!(
        state.ambiguous_protocols.get(&foreign_protocol),
        Some(&foreign_marker)
    );
    drop(state);
    foreign
        .settle_incomplete(DurableSettlement::Incomplete)
        .unwrap();
    clear_protocol(&foreign_protocol);
}

fn clear_protocol(protocol: &str) {
    let mut state = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    state.ambiguous_protocols.remove(protocol);
    state
        .failure_records
        .retain(|_, record| record.protocol_id != protocol);
}
