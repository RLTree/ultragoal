use super::*;

impl<'a> EventSelection<'a> {
    pub(crate) const fn empty() -> Self {
        Self {
            selected: None,
            matched_reference_count: 0,
            eligible_failure_count: 0,
        }
    }

    pub(crate) fn from_events(events: &'a [SemanticEvent], finding: &Finding) -> Self {
        let matches = events
            .iter()
            .filter(|event| references_finding(event, finding))
            .collect::<Vec<_>>();
        let eligible = matches
            .iter()
            .copied()
            .filter(|event| is_causal_failure(event))
            .collect::<Vec<_>>();
        Self {
            selected: eligible.last().copied().or_else(|| matches.last().copied()),
            matched_reference_count: matches.len(),
            eligible_failure_count: eligible.len(),
        }
    }

    pub(crate) fn target_id(self) -> &'a str {
        self.selected
            .map(SemanticEvent::event_id)
            .unwrap_or(MISSING_EVENT_ID)
    }

    pub(crate) fn provenance(self) -> Value {
        let selected = self.selected.map(|event| {
            json!({
                "event_id": event.event_id(),
                "parent_event_id": event.parent_event_id(),
                "source_id": event.source_id(),
                "operation": event.operation(),
                "outcome": event.outcome(),
                "observed_at_unix_ms": event.observed_at_unix_ms(),
                "sequence": event.sequence(),
                "causal_failure_eligible": is_causal_failure(event)
            })
        });
        json!({
            "schema_version": "ObservedFailureProvenance-v1",
            "selection_rule": "latest-bounded-causal-failure-then-latest-reference",
            "matched_reference_count": self.matched_reference_count,
            "eligible_failure_count": self.eligible_failure_count,
            "selected": selected,
            "claim_effect": "none"
        })
    }
}

pub(crate) fn references_finding(event: &SemanticEvent, finding: &Finding) -> bool {
    event.finding_refs().contains(&finding.finding_id)
        || event.repair_refs().contains(&finding.repair.repair_id)
}

pub(crate) fn is_causal_failure(event: &SemanticEvent) -> bool {
    matches!(event.outcome(), "fail" | "error" | "blocked" | "cancelled")
        && !event.operation().starts_with("receipt.")
        && !event.operation().starts_with("telemetry.")
        && !event.source_id().contains("receipt")
        && !event.source_id().contains("telemetry")
}

#[derive(Clone, Copy)]
pub(crate) struct QueryWindow {
    pub(crate) evaluated: bool,
    pub(crate) result_count: usize,
    pub(crate) saturated: bool,
}

impl QueryWindow {
    pub(crate) fn from_events(events: &[SemanticEvent]) -> Self {
        Self {
            evaluated: true,
            result_count: events.len(),
            saturated: events.len() == EventStore::supported_result_limit(),
        }
    }

    pub(crate) const fn empty() -> Self {
        Self {
            evaluated: true,
            result_count: 0,
            saturated: false,
        }
    }

    pub(crate) const fn not_evaluated() -> Self {
        Self {
            evaluated: false,
            result_count: 0,
            saturated: false,
        }
    }

    pub(crate) const fn unavailable() -> Self {
        Self {
            evaluated: true,
            result_count: 0,
            saturated: false,
        }
    }

    pub(crate) fn value(self) -> Value {
        json!({
            "schema_version": "BoundedLocalQueryWindow-v1",
            "evaluated": self.evaluated,
            "result_count": self.result_count,
            "result_limit": EventStore::supported_result_limit(),
            "saturated": self.saturated,
            "ordering": "observed_at_unix_ms-sequence-event_id"
        })
    }
}
