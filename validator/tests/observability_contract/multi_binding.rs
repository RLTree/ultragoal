use super::scenario::{TestDir, event, query, store};
use ultragoal::observability::{EventQuery, EventStore, SemanticEvent, SemanticEventInput};

fn event_for(
    context: &str,
    candidate: &str,
    source: &str,
    id: &str,
    parent: Option<&str>,
) -> SemanticEvent {
    let mut event = SemanticEvent::new(SemanticEventInput {
        context_id: context.to_owned(),
        candidate_id: candidate.to_owned(),
        source_id: source.to_owned(),
        event_id: id.to_owned(),
        observed_at_unix_ms: 1,
        sequence: 1,
        operation: "check.run".to_owned(),
        outcome: "fail".to_owned(),
    })
    .unwrap();
    if let Some(parent) = parent {
        event.set_parent(parent).unwrap();
    }
    event
}

#[test]
fn valid_historical_bindings_coexist_without_entering_current_queries() {
    let dir = TestDir::new("multi-binding");
    let historical =
        EventStore::open_bound(dir.store_path(), "ctx-old", "cand-old", "source-1").unwrap();
    historical
        .append(&event_for(
            "ctx-old",
            "cand-old",
            "source-1",
            "historical",
            None,
        ))
        .unwrap();

    let current = store(&dir);
    assert!(current.query(&query()).unwrap().is_empty());
    assert!(current.append(&event("current", 2, 2, "fail")).unwrap());
    assert_eq!(current.query(&query()).unwrap()[0].event_id(), "current");

    let historical_query = EventQuery::new("ctx-old", "cand-old", "source-1").unwrap();
    assert_eq!(
        historical.query(&historical_query).unwrap()[0].event_id(),
        "historical"
    );
}

#[test]
fn causal_explanation_cannot_cross_a_binding_boundary() {
    let dir = TestDir::new("multi-binding-cause");
    let historical =
        EventStore::open_bound(dir.store_path(), "ctx-old", "cand-old", "source-1").unwrap();
    historical
        .append(&event_for(
            "ctx-old",
            "cand-old",
            "source-1",
            "historical-parent",
            None,
        ))
        .unwrap();

    let current = store(&dir);
    current
        .append(&event_for(
            "ctx-1",
            "cand-1",
            "source-1",
            "current-child",
            Some("historical-parent"),
        ))
        .unwrap();
    let explanation = current.explain(&query(), "current-child").unwrap();
    assert_eq!(explanation.classification(), "missing-evidence");
    assert_eq!(
        explanation.diagnostic_code(),
        "observe-evidence-missing:causal-parent"
    );
}
