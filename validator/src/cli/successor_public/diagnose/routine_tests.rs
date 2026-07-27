use super::*;
use crate::cli::successor_public::routine::RoutineCheckpointProjection;

fn checkpoint(state: &str, outcome: Option<&str>) -> RoutineCheckpointProjection {
    RoutineCheckpointProjection {
        state: state.to_owned(),
        terminal_outcome: outcome.map(str::to_owned),
        event_id: "event-id".to_owned(),
        event_status: "status".to_owned(),
        event_transition: "transition".to_owned(),
        finding_id: None,
    }
}

#[test]
fn checkpoint_states_have_distinct_effect_and_recovery_dispositions() {
    let cases = [
        ("reserved", None, "outcome_unknown", "ambiguous", "withheld"),
        (
            "reconciled",
            None,
            "no_effect_reconciled",
            "no_effect",
            "honest_rerun",
        ),
        (
            "ambiguous",
            Some("ambiguous"),
            "ambiguous",
            "ambiguous",
            "withheld",
        ),
        (
            "terminal-event-pending",
            Some("complete"),
            "settlement_pending",
            "committed",
            "withheld_until_settlement",
        ),
        (
            "terminal-event-pending",
            Some("failed"),
            "terminal_failure",
            "committed",
            "honest_rerun",
        ),
        (
            "terminal-event-pending",
            Some("cancelled"),
            "terminal_failure",
            "committed",
            "honest_rerun",
        ),
        (
            "terminal-event-pending",
            Some("incomplete"),
            "terminal_failure",
            "committed",
            "honest_rerun",
        ),
        (
            "terminal-event-joined",
            Some("failed"),
            "terminal_failure",
            "committed",
            "honest_rerun",
        ),
        (
            "terminal-event-joined",
            Some("complete"),
            "complete",
            "committed",
            "safe_reuse",
        ),
    ];
    for (state, outcome, expected_status, effect, reuse) in cases {
        let checkpoint = checkpoint(state, outcome);
        let result = disposition(Some(&checkpoint));
        assert_eq!(result.status, expected_status);
        assert_eq!(result.effect_state, effect);
        assert_eq!(result.repeat_use, reuse);
    }
}

#[test]
fn missing_checkpoint_is_no_effect_without_claim_promotion() {
    let result = disposition(None);
    assert_eq!(result.status, "no_record");
    assert_eq!(result.effect_state, "no_effect");
    assert_eq!(result.repeat_use, "not_available");
    assert_eq!(result.exit, ExitClass::ActionableFinding);
}
