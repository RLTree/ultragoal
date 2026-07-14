fn terminal_reobservation_guard_is_complete(source: &str) -> bool {
    let Some(start) = source.find("fn reobserve_terminal_failure") else {
        return false;
    };
    let Some(end_offset) = source[start..].find("fn finish_publication_failure") else {
        return false;
    };
    let window = &source[start..start + end_offset];
    let required_positions = [
        "head_before != head_after || record_before != record_after",
        "head_after != *record_after.current_head()",
        "record_after.reservation().permit_id() != effect.permit().permit_id()",
        "record_after == *effect.record()",
        "record_after.reservation() == effect.record().reservation()",
    ]
    .map(|token| window.find(token));
    let [
        Some(stability),
        Some(head_coherence),
        Some(permit_coherence),
        Some(still_in_flight),
        Some(terminal),
    ] = required_positions
    else {
        return false;
    };
    stability < head_coherence
        && head_coherence < still_in_flight
        && head_coherence < terminal
        && permit_coherence < still_in_flight
        && permit_coherence < terminal
}

fn post_reservation_reobservation_guard_is_complete(source: &str) -> bool {
    let Some(start) = source.find("fn reobserve_post_reservation") else {
        return false;
    };
    let rest = &source[start..];
    let end_offset = rest.find("#[cfg(test)]").unwrap_or(rest.len());
    let window = &rest[..end_offset];
    let required_positions = [
        "head_before != head_after || record_before != record_after",
        "head_after != *record_after.current_head()",
        "record_after.reservation().permit_id() != effect.permit().permit_id()",
        "classification: HostEffectPostReservationLedgerClassification::StillInFlight",
        "classification: HostEffectPostReservationLedgerClassification::TerminalObserved",
        "classification: HostEffectPostReservationLedgerClassification::ObservationRejected",
    ]
    .map(|token| window.find(token));
    let [
        Some(stability),
        Some(head_coherence),
        Some(permit_coherence),
        Some(still_in_flight),
        Some(terminal),
        Some(rejected),
    ] = required_positions
    else {
        return false;
    };
    stability < head_coherence
        && stability < permit_coherence
        && head_coherence < still_in_flight
        && head_coherence < terminal
        && head_coherence < rejected
        && permit_coherence < still_in_flight
        && permit_coherence < terminal
        && permit_coherence < rejected
}
