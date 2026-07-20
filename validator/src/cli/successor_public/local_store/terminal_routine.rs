use super::*;
use sha2::{Digest, Sha256};

/// A terminal routine observation. It is diagnostic-only: it names neither a
/// finding nor a claim and cannot change product-state authority.
pub(crate) struct RoutineTerminalEvent<'a> {
    pub(crate) event_id: &'a str,
    pub(crate) continuation_id: &'a str,
    pub(crate) terminal_ledger_head: &'a str,
    pub(crate) observed_at_unix_ms: u64,
    pub(crate) sequence: u64,
    pub(crate) parent_event_id: Option<&'a str>,
    pub(crate) status: &'a str,
    pub(crate) transition: &'a str,
    pub(crate) terminal_outcome: crate::routine_work::RoutineTerminalOutcome,
    pub(crate) finding_binding: Option<&'a crate::state::RoutineFindingBinding>,
}

pub(crate) fn terminal_event_id(continuation_id: &str, terminal_ledger_head: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(b"routine-terminal-event-v1\0");
    digest.update(continuation_id.as_bytes());
    digest.update(b"\0");
    digest.update(terminal_ledger_head.as_bytes());
    format!("routine-terminal-{:x}", digest.finalize())
}

pub(crate) fn routine_observations_from_events(
    events: &[SemanticEvent],
) -> Vec<crate::state::RoutineFindingObservation> {
    events
        .iter()
        .filter_map(|event| {
            if event.operation() != "check.routine.terminal" {
                return None;
            }
            let attributes = event.public_attributes();
            let transition = match attributes.get("routine_transition")?.as_str() {
                "interrupted" => crate::state::RoutineObservationTransition::Interrupted,
                "recovered" => crate::state::RoutineObservationTransition::Recovered,
                "reused" => crate::state::RoutineObservationTransition::Reused,
                "executed" => crate::state::RoutineObservationTransition::Executed,
                "failed" => crate::state::RoutineObservationTransition::Failed,
                "cancelled" => crate::state::RoutineObservationTransition::Cancelled,
                _ => return None,
            };
            let mut finding_ids = event.finding_refs().iter();
            let mut repair_ids = event.repair_refs().iter();
            let finding_id = finding_ids.next()?.to_owned();
            let repair_id = repair_ids.next()?.to_owned();
            if finding_ids.next().is_some() || repair_ids.next().is_some() {
                return None;
            }
            Some(crate::state::RoutineFindingObservation {
                event_id: event.event_id().to_owned(),
                continuation_id: attributes.get("continuation_id")?.to_owned(),
                terminal_ledger_head: attributes.get("terminal_ledger_head")?.to_owned(),
                finding_id,
                repair_id,
                transition,
                outcome: attributes.get("routine_terminal_outcome")?.to_owned(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{routine_observations_from_events, terminal_event_id};
    use crate::observability::{SemanticEvent, SemanticEventInput};

    fn event() -> SemanticEvent {
        SemanticEvent::new(SemanticEventInput {
            context_id: "sha256:context".to_owned(),
            candidate_id: "sha256:candidate".to_owned(),
            source_id: "successor-runtime".to_owned(),
            event_id: "routine-terminal-test".to_owned(),
            observed_at_unix_ms: 1,
            sequence: 1,
            operation: "check.routine.terminal".to_owned(),
            outcome: "pass".to_owned(),
        })
        .unwrap()
    }

    #[test]
    fn terminal_event_id_is_bound_only_to_continuation_and_terminal_head() {
        assert_eq!(
            terminal_event_id("routine-cont-a", "head-a"),
            terminal_event_id("routine-cont-a", "head-a")
        );
        assert_ne!(
            terminal_event_id("routine-cont-a", "head-a"),
            terminal_event_id("routine-cont-b", "head-a")
        );
        assert_ne!(
            terminal_event_id("routine-cont-a", "head-a"),
            terminal_event_id("routine-cont-a", "head-b")
        );
    }

    #[test]
    fn observation_requires_one_persisted_finding_and_repair_binding() {
        let mut unbound = event();
        unbound
            .add_public_attribute("continuation_id", "routine-cont-test")
            .unwrap();
        unbound
            .add_public_attribute("terminal_ledger_head", "sha256:head")
            .unwrap();
        unbound
            .add_public_attribute("routine_transition", "executed")
            .unwrap();
        unbound
            .add_public_attribute("routine_terminal_outcome", "complete")
            .unwrap();
        assert!(routine_observations_from_events(&[unbound]).is_empty());

        let mut bound = event();
        for (key, value) in [
            ("continuation_id", "routine-cont-test"),
            ("terminal_ledger_head", "sha256:head"),
            ("routine_transition", "executed"),
            ("routine_terminal_outcome", "complete"),
        ] {
            bound.add_public_attribute(key, value).unwrap();
        }
        bound.add_finding_ref("sha256:finding").unwrap();
        bound.add_repair_ref("repair-test").unwrap();
        let observations = routine_observations_from_events(&[bound]);
        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].finding_id, "sha256:finding");
        assert_eq!(observations[0].repair_id, "repair-test");
        assert_eq!(observations[0].outcome, "complete");

        let mut ambiguous = event();
        for (key, value) in [
            ("continuation_id", "routine-cont-test"),
            ("terminal_ledger_head", "sha256:head"),
            ("routine_transition", "executed"),
            ("routine_terminal_outcome", "complete"),
        ] {
            ambiguous.add_public_attribute(key, value).unwrap();
        }
        ambiguous.add_finding_ref("sha256:finding-a").unwrap();
        ambiguous.add_finding_ref("sha256:finding-b").unwrap();
        ambiguous.add_repair_ref("repair-a").unwrap();
        ambiguous.add_repair_ref("repair-b").unwrap();
        assert!(routine_observations_from_events(&[ambiguous]).is_empty());
    }
}
