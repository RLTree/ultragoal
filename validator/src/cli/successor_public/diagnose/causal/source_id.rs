use super::*;

pub(crate) const SOURCE_ID: &str = "successor-runtime";
pub(crate) const MISSING_EVENT_ID: &str = "diagnose-missing-event";

pub(crate) fn evaluate(root: &Path, context: &LiveContext, finding: &Finding) -> Value {
    let store = match LocalStore::open(root, context, SOURCE_ID) {
        Ok(store) => store,
        Err(failure) => return unavailable(failure, EventSelection::empty()),
    };
    let query = match EventQuery::for_context(context, SOURCE_ID)
        .and_then(|query| query.limit(EventStore::supported_result_limit()))
    {
        Ok(query) => query,
        Err(_) => return unavailable(LocalStoreFailure::binding(), EventSelection::empty()),
    };
    if store.status() == "absent" {
        return projection(
            "absent",
            missing_store(),
            EventSelection::empty(),
            QueryWindow::empty(),
            None,
        );
    }
    let events = match store.query_diagnostic(&query) {
        Ok(events) => events,
        Err(failure) if failure.is_corruption() => {
            return corruption_projection(&store, &query, failure);
        }
        Err(failure) => return unavailable(failure, EventSelection::empty()),
    };
    let window = QueryWindow::from_events(&events);
    if window.saturated {
        if let Err(failure) = store.revalidate() {
            return unavailable(failure, EventSelection::empty());
        }
        return projection(
            "available",
            incomplete_window(),
            EventSelection::empty(),
            window,
            None,
        );
    }
    let selection = EventSelection::from_events(&events, finding);
    let explanation = match store.explain(&query, selection.target_id()) {
        Ok(Some(explanation)) => match serde_json::to_value(explanation) {
            Ok(value) => value,
            Err(_) => {
                return unavailable(LocalStoreFailure::projection(), selection);
            }
        },
        Ok(None) => return unavailable(LocalStoreFailure::missing_store(), selection),
        Err(failure) => return unavailable(failure, selection),
    };
    if let Err(failure) = store.revalidate() {
        return unavailable(failure, selection);
    }
    projection("available", explanation, selection, window, None)
}

pub(crate) fn not_evaluated() -> Value {
    projection(
        "not_opened",
        json!({
            "schema_version": "CausalExplanation-v1",
            "classification": "not-evaluated",
            "summary": "Select one current finding to correlate it with candidate-bound local events.",
            "causal_event_ids": [],
            "diagnostic_code": "observe-cause-not-evaluated:finding-required",
            "repair": "Run diagnose with one current --finding identifier.",
            "claim_effect": "none"
        }),
        EventSelection::empty(),
        QueryWindow::not_evaluated(),
        None,
    )
}

pub(crate) fn corruption_projection(
    store: &LocalStore,
    query: &EventQuery,
    failure: LocalStoreFailure,
) -> Value {
    let explanation = match store.explain(query, MISSING_EVENT_ID) {
        Ok(Some(explanation)) => serde_json::to_value(explanation).ok(),
        _ => None,
    };
    if let Some(explanation) = explanation
        && store.revalidate().is_ok()
    {
        return projection(
            "available",
            explanation,
            EventSelection::empty(),
            QueryWindow::unavailable(),
            Some(failure),
        );
    }
    unavailable(failure, EventSelection::empty())
}

pub(crate) fn unavailable(failure: LocalStoreFailure, selection: EventSelection<'_>) -> Value {
    projection(
        "unavailable",
        json!({
            "schema_version": "CausalExplanation-v1",
            "classification": "unavailable",
            "summary": failure.summary(),
            "causal_event_ids": [],
            "diagnostic_code": failure.diagnostic_code(),
            "repair": failure.repair(),
            "claim_effect": "none"
        }),
        selection,
        QueryWindow::unavailable(),
        Some(failure),
    )
}

pub(crate) fn projection(
    store_status: &str,
    explanation: Value,
    selection: EventSelection<'_>,
    window: QueryWindow,
    read_failure: Option<LocalStoreFailure>,
) -> Value {
    json!({
        "schema_version": "PublicCausalDiagnosis-v1",
        "store_status": store_status,
        "matched_event_id": selection.selected.map(SemanticEvent::event_id),
        "failure_provenance": selection.provenance(),
        "query_window": window.value(),
        "read_failure": read_failure.map(LocalStoreFailure::value),
        "explanation": explanation,
        "policy": local_policy(),
        "claim_effect": "none"
    })
}

pub(crate) fn missing_store() -> Value {
    json!({
        "schema_version": "CausalExplanation-v1",
        "classification": "missing-evidence",
        "summary": "No local event store exists for this candidate; causality is withheld.",
        "causal_event_ids": [],
        "diagnostic_code": "observe-evidence-missing:store-empty",
        "repair": "Run the affected operation with bounded local event emission, then diagnose the current finding again.",
        "claim_effect": "none"
    })
}

pub(crate) fn incomplete_window() -> Value {
    json!({
        "schema_version": "CausalExplanation-v1",
        "classification": "incomplete-evidence",
        "summary": "The bounded local result window is full, so a globally current causal event cannot be selected safely.",
        "causal_event_ids": [],
        "diagnostic_code": "observe-evidence-incomplete:query-result-limit",
        "repair": "Narrow the local event set through explicit lifecycle policy, then rerun diagnosis against the current candidate.",
        "claim_effect": "none"
    })
}

#[derive(Clone, Copy)]
pub(crate) struct EventSelection<'a> {
    pub(crate) selected: Option<&'a SemanticEvent>,
    pub(crate) matched_reference_count: usize,
    pub(crate) eligible_failure_count: usize,
}
