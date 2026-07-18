use super::observability::{EventQuery, EventStore, SemanticEvent, SemanticEventInput};
use super::scenario::{TestDir, event, query, store};

#[test]
fn attribute_row_event_store_scan_and_result_limits_fail_closed() {
    let mut high_cardinality = event("attrs", 1, 1, "fail");
    for index in 0..32 {
        high_cardinality
            .add_public_attribute(&format!("key{index}"), "safe")
            .unwrap();
    }
    assert!(
        high_cardinality
            .add_public_attribute("overflow", "safe")
            .unwrap_err()
            .contains("attribute-limit")
    );

    let mut oversized = event("oversized", 1, 1, "fail");
    assert!(
        oversized
            .add_public_attribute("payload", &"x".repeat(513))
            .unwrap_err()
            .contains("attribute-limit")
    );

    let dir = TestDir::new("event-limit");
    let limited = store(&dir).with_event_limit(1).unwrap();
    limited.append(&event("one", 1, 1, "pass")).unwrap();
    assert!(
        limited
            .append(&event("two", 2, 2, "pass"))
            .unwrap_err()
            .contains("event-limit")
    );

    let dir = TestDir::new("store-limit");
    let limited = store(&dir).with_store_limit(600).unwrap();
    let mut padded = event("padded", 1, 1, "pass");
    padded
        .add_public_attribute("payload", &"x".repeat(300))
        .unwrap();
    assert!(limited.append(&padded).unwrap_err().contains("store-limit"));

    let dir = TestDir::new("scan-limit");
    let full = store(&dir);
    full.append(&event("one", 1, 1, "pass")).unwrap();
    full.append(&event("two", 2, 2, "pass")).unwrap();
    assert!(
        store(&dir)
            .with_scan_limit(1)
            .unwrap()
            .query(&query())
            .unwrap_err()
            .contains("scan-limit")
    );

    let capped_query = query().limit(2).unwrap();
    assert!(
        store(&dir)
            .with_result_limit(1)
            .unwrap()
            .query(&capped_query)
            .unwrap_err()
            .contains("query-limit")
    );

    let row = std::fs::read(dir.store_path()).unwrap();
    std::fs::write(dir.store_path(), row.repeat(2)).unwrap();
    assert!(
        store(&dir)
            .with_event_limit(1)
            .unwrap()
            .query(&query())
            .unwrap_err()
            .contains("event-limit")
    );
}

#[test]
fn arithmetic_overflow_and_invalid_query_ranges_are_rejected() {
    let mut event = event("overflow", 1, 1, "pass");
    assert!(event.set_work_counts(u64::MAX, u64::MAX, 1).is_err());
    assert!(event.set_work_counts(1, 1, 1).is_err());
    assert!(
        EventQuery::new("ctx-1", "cand-1", "source-1")
            .unwrap()
            .since(2)
            .until(1)
            .is_err()
    );
    assert!(query().limit(0).is_err());
    assert!(
        store(&TestDir::new("bad-bound"))
            .with_store_limit(u64::MAX)
            .is_err()
    );
}

#[test]
fn claim_like_fields_and_receipt_only_rows_are_explicit_false_pass_controls() {
    let mut forged = event("forged", 1, 1, "pass");
    for key in [
        "claim_status",
        "readiness",
        "completion",
        "supported-claims",
    ] {
        assert!(
            forged
                .add_public_attribute(key, "pass")
                .unwrap_err()
                .contains("claim-authority")
        );
    }
    assert!(
        forged
            .add_sensitive_attribute("claim_status", "pass")
            .unwrap_err()
            .contains("claim-authority")
    );

    let dir = TestDir::new("receipt-only");
    let receipt = SemanticEvent::new(SemanticEventInput {
        context_id: "ctx-1".to_owned(),
        candidate_id: "cand-1".to_owned(),
        source_id: "receipt-source".to_owned(),
        event_id: "receipt-row".to_owned(),
        observed_at_unix_ms: 1,
        sequence: 1,
        operation: "receipt.present".to_owned(),
        outcome: "fail".to_owned(),
    })
    .unwrap();
    let receipt_store =
        EventStore::open_bound(dir.store_path(), "ctx-1", "cand-1", "receipt-source").unwrap();
    receipt_store.append(&receipt).unwrap();
    let receipt_query = EventQuery::new("ctx-1", "cand-1", "receipt-source").unwrap();
    let explanation = receipt_store
        .explain(&receipt_query, "receipt-row")
        .unwrap();
    assert_eq!(explanation.classification(), "missing-evidence");
    assert_eq!(
        explanation.diagnostic_code(),
        "observe-evidence-missing:receipt-only"
    );
    assert!(!explanation.can_raise_claim());
}

#[test]
fn explicit_clear_is_bounded_and_missing_clear_is_a_no_op() {
    let dir = TestDir::new("clear");
    let store = store(&dir);
    assert!(!store.clear().unwrap());
    store.append(&event("one", 1, 1, "pass")).unwrap();
    assert!(store.clear().unwrap());
    assert!(store.query(&query()).unwrap().is_empty());
}
