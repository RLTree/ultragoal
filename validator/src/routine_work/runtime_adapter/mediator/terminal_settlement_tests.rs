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
        seed(
            &reservation,
            &reservation.grant_id,
            &reservation.recovery_marker,
        );
        let marker = reservation.settle_incomplete(outcome).unwrap();
        assert_eq!(marker, None);
        let state = registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert!(
            !state
                .active_protocols
                .contains_key(&reservation.protocol_id)
        );
        assert!(
            !state
                .ambiguous_protocols
                .contains_key(&reservation.protocol_id)
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
fn non_durable_missing_or_foreign_ambiguity_emits_no_marker() {
    for (label, ambiguity) in [("missing", None), ("foreign", Some("foreign-marker"))] {
        let reservation = attempt(label, None, true, None);
        let mut state = registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.active_protocols.insert(
            reservation.protocol_id.clone(),
            reservation.grant_id.clone(),
        );
        if let Some(ambiguity) = ambiguity {
            state
                .ambiguous_protocols
                .insert(reservation.protocol_id.clone(), ambiguity.to_owned());
        }
        drop(state);
        assert_eq!(
            reservation
                .settle_incomplete(DurableSettlement::Incomplete)
                .unwrap(),
            None
        );
        let mut state = registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert_eq!(
            state
                .ambiguous_protocols
                .get(&reservation.protocol_id)
                .map(String::as_str),
            ambiguity
        );
        state.ambiguous_protocols.remove(&reservation.protocol_id);
    }
}

#[test]
fn non_durable_ambiguity_retains_the_exact_pending_marker() {
    let reservation = attempt("non-durable", None, true, None);
    seed(
        &reservation,
        &reservation.grant_id,
        &reservation.recovery_marker,
    );
    assert_eq!(
        reservation
            .settle_incomplete(DurableSettlement::Incomplete)
            .unwrap(),
        Some(reservation.recovery_marker.clone())
    );
    let mut state = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(
        state.ambiguous_protocols.get(&reservation.protocol_id),
        Some(&reservation.recovery_marker)
    );
    state.ambiguous_protocols.remove(&reservation.protocol_id);
}

#[test]
fn terminal_cleanup_preserves_foreign_protocol_marker_and_grant() {
    let durable = Arc::new(TerminalDurable::default());
    let reservation = attempt("foreign", Some(durable), true, None);
    let foreign_protocol = "terminal-protocol-foreign-other".to_owned();
    seed(&reservation, "foreign-grant", "foreign-marker");
    {
        let mut state = registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.ambiguous_protocols.insert(
            foreign_protocol.clone(),
            reservation.recovery_marker.clone(),
        );
    }
    assert_eq!(
        reservation
            .settle_incomplete(DurableSettlement::Failed)
            .unwrap(),
        None
    );
    let mut state = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(
        state.active_protocols.get(&reservation.protocol_id),
        Some(&"foreign-grant".to_owned())
    );
    assert_eq!(
        state.ambiguous_protocols.get(&reservation.protocol_id),
        Some(&"foreign-marker".to_owned())
    );
    assert_eq!(
        state.ambiguous_protocols.get(&foreign_protocol),
        Some(&reservation.recovery_marker)
    );
    state.active_protocols.remove(&reservation.protocol_id);
    state.ambiguous_protocols.remove(&reservation.protocol_id);
    state.ambiguous_protocols.remove(&foreign_protocol);
}
