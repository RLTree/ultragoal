use super::privacy;
use super::{EventQuery, EventStore, SemanticEvent};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CausalExplanation {
    schema_version: &'static str,
    classification: &'static str,
    summary: String,
    causal_event_ids: Vec<String>,
    diagnostic_code: String,
    repair: String,
    claim_effect: &'static str,
}

impl CausalExplanation {
    pub fn classification(&self) -> &str {
        self.classification
    }
    pub fn summary(&self) -> &str {
        &self.summary
    }
    pub fn causal_event_ids(&self) -> &[String] {
        &self.causal_event_ids
    }
    pub fn diagnostic_code(&self) -> &str {
        &self.diagnostic_code
    }
    pub fn repair(&self) -> &str {
        &self.repair
    }
    pub fn can_raise_claim(&self) -> bool {
        false
    }
}

pub(super) fn explain(
    store: &EventStore,
    query: &EventQuery,
    target_event_id: &str,
) -> Result<CausalExplanation, String> {
    privacy::validate_identifier("target-event-id", target_event_id)?;
    if let Err(error) = store.validate_query_binding(query) {
        if error.starts_with("observe-binding-") {
            return Ok(stale_context(&error));
        }
        return Err(error);
    }
    let events = match store.read_events() {
        Ok(events) => events,
        Err(error) if error.starts_with("observe-store-corrupt:") => {
            return Ok(corruption(&error));
        }
        Err(error) => return Err(error),
    };
    if events.is_empty() {
        return Ok(missing("observe-evidence-missing:store-empty", Vec::new()));
    }
    let by_id: BTreeMap<&str, &SemanticEvent> = events
        .iter()
        .map(|event| (event.event_id(), event))
        .collect();
    let Some(target) = by_id.get(target_event_id).copied() else {
        return Ok(missing("observe-evidence-missing:target-event", Vec::new()));
    };
    if !query.matches(target) {
        return Ok(missing(
            "observe-evidence-missing:target-outside-query",
            Vec::new(),
        ));
    }
    if target.is_receipt_only() {
        return Ok(missing(
            "observe-evidence-missing:receipt-only",
            vec![target.event_id().to_owned()],
        ));
    }
    if !target.is_failure() {
        return Ok(missing(
            "observe-evidence-missing:target-not-failure",
            vec![target.event_id().to_owned()],
        ));
    }
    causal_chain(target, &by_id)
}

fn causal_chain(
    target: &SemanticEvent,
    by_id: &BTreeMap<&str, &SemanticEvent>,
) -> Result<CausalExplanation, String> {
    let mut seen = BTreeSet::new();
    let mut chain = Vec::new();
    let mut current = target;
    loop {
        if !seen.insert(current.event_id()) {
            return Ok(corruption("observe-store-corrupt:causal-cycle"));
        }
        chain.push(current.event_id().to_owned());
        let Some(parent_id) = current.parent_event_id() else {
            break;
        };
        let Some(parent) = by_id.get(parent_id).copied() else {
            chain.reverse();
            return Ok(missing("observe-evidence-missing:causal-parent", chain));
        };
        current = parent;
    }
    chain.reverse();
    Ok(CausalExplanation {
        schema_version: "CausalExplanation-v1",
        classification: "observed-cause",
        summary: "An explicit context-bound parent chain identifies the observed root event; no unlinked cause was inferred.".to_owned(),
        causal_event_ids: chain,
        diagnostic_code: "observe-cause-explicit-parent-chain".to_owned(),
        repair: "Apply the repair bound to the root finding, then rerun the affected operation and append a new event.".to_owned(),
        claim_effect: "none",
    })
}

fn missing(code: &str, causal_event_ids: Vec<String>) -> CausalExplanation {
    CausalExplanation {
        schema_version: "CausalExplanation-v1",
        classification: "missing-evidence",
        summary: "The local store does not contain an explicit causal chain for this target; causality is withheld.".to_owned(),
        causal_event_ids,
        diagnostic_code: code.to_owned(),
        repair: "Emit bounded context-matched semantic events with explicit parent links, then rerun the query.".to_owned(),
        claim_effect: "none",
    }
}

fn corruption(code: &str) -> CausalExplanation {
    CausalExplanation {
        schema_version: "CausalExplanation-v1",
        classification: "corruption",
        summary: "The event store failed integrity validation; its rows are not trusted for diagnosis.".to_owned(),
        causal_event_ids: Vec::new(),
        diagnostic_code: code.to_owned(),
        repair: "Preserve the file for review; recover only a truncated tail or replace it from a separately verified source.".to_owned(),
        claim_effect: "none",
    }
}

fn stale_context(code: &str) -> CausalExplanation {
    let classification = if code.contains("stale-") {
        "stale-context"
    } else {
        "wrong-context"
    };
    CausalExplanation {
        schema_version: "CausalExplanation-v1",
        classification,
        summary: "The query binding does not match the current store context, candidate, or source; rows were not interpreted.".to_owned(),
        causal_event_ids: Vec::new(),
        diagnostic_code: code.to_owned(),
        repair: "Recompute live context and candidate identity, then select the separately bound store and rerun.".to_owned(),
        claim_effect: "none",
    }
}
