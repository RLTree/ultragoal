use super::scenario::{TestDir, event, query, store};
use std::fs::{self, OpenOptions};
use std::io::Write;
use ultragoal::observability::{EventQuery, EventStore, SemanticEvent, SemanticEventInput};

#[test]
fn malformed_unknown_version_checksum_and_truncation_fail_closed() {
    let cases = [
        (
            "malformed",
            b"{not-json}\n".as_slice(),
            "malformed-or-unknown",
        ),
        (
            "truncated",
            b"{\"row_version\":".as_slice(),
            "truncated-tail",
        ),
    ];
    for (label, bytes, expected) in cases {
        let dir = TestDir::new(label);
        fs::write(dir.store_path(), bytes).unwrap();
        let error = store(&dir).query(&query()).unwrap_err();
        assert!(error.contains(expected), "{label}: {error}");
        let explanation = store(&dir).explain(&query(), "event").unwrap();
        assert_eq!(explanation.classification(), "corruption");
    }

    let dir = TestDir::new("unknown-row-field");
    let row_field_store = store(&dir);
    row_field_store
        .append(&event("event", 1, 1, "fail"))
        .unwrap();
    mutate_text(&dir, |text| text.replace("}\n", ",\"unknown\":true}\n"));
    assert!(
        row_field_store
            .query(&query())
            .unwrap_err()
            .contains("malformed-or-unknown")
    );

    let dir = TestDir::new("unknown-row-version");
    let version_store = store(&dir);
    version_store.append(&event("event", 1, 1, "fail")).unwrap();
    mutate_text(&dir, |text| {
        text.replace("SemanticEventRow-v1", "SemanticEventRow-v9")
    });
    assert!(
        version_store
            .query(&query())
            .unwrap_err()
            .contains("unsupported-version")
    );

    let dir = TestDir::new("checksum-tamper");
    let checksum_store = store(&dir);
    checksum_store
        .append(&event("event", 1, 1, "fail"))
        .unwrap();
    mutate_text(&dir, |text| text.replace("\"fail\"", "\"pass\""));
    assert!(
        checksum_store
            .query(&query())
            .unwrap_err()
            .contains("checksum")
    );
}

#[test]
fn unknown_event_fields_are_rejected_even_with_valid_row_shape() {
    let dir = TestDir::new("unknown-event-field");
    let event_field_store = store(&dir);
    event_field_store
        .append(&event("event", 1, 1, "fail"))
        .unwrap();
    mutate_text(&dir, |text| {
        text.replacen(
            "\"schema_version\":",
            "\"unknown_event\":true,\"schema_version\":",
            1,
        )
    });
    assert!(
        event_field_store
            .query(&query())
            .unwrap_err()
            .contains("malformed-or-unknown")
    );
}

#[test]
fn wrong_context_candidate_and_source_are_rejected_on_append_query_and_row_read() {
    let dir = TestDir::new("bindings");
    let bound_store = store(&dir);
    let wrong = SemanticEvent::new(SemanticEventInput {
        context_id: "ctx-1".to_owned(),
        candidate_id: "cand-1".to_owned(),
        source_id: "source-2".to_owned(),
        event_id: "wrong-source".to_owned(),
        observed_at_unix_ms: 1,
        sequence: 1,
        operation: "check.run".to_owned(),
        outcome: "fail".to_owned(),
    })
    .unwrap();
    assert!(
        bound_store
            .append(&wrong)
            .unwrap_err()
            .contains("wrong-source")
    );

    for (query, classification) in [
        (
            EventQuery::new("ctx-2", "cand-1", "source-1").unwrap(),
            "wrong-context",
        ),
        (
            EventQuery::new("ctx-1", "cand-2", "source-1").unwrap(),
            "stale-context",
        ),
        (
            EventQuery::new("ctx-1", "cand-1", "source-2").unwrap(),
            "wrong-context",
        ),
    ] {
        assert!(bound_store.query(&query).is_err());
        assert_eq!(
            bound_store
                .explain(&query, "missing")
                .unwrap()
                .classification(),
            classification
        );
    }

    let other_dir = TestDir::new("bindings-other");
    let other =
        EventStore::open_bound(other_dir.store_path(), "ctx-1", "cand-1", "source-2").unwrap();
    other.append(&wrong).unwrap();
    fs::write(dir.store_path(), fs::read(other_dir.store_path()).unwrap()).unwrap();
    assert!(
        store(&dir)
            .query(&query())
            .unwrap_err()
            .contains("row-binding-conflict")
    );
}

#[test]
fn conflicting_ids_reject_while_physical_duplicates_collapse_and_reordering_is_stable() {
    let dir = TestDir::new("duplicates-reorder");
    let duplicate_store = store(&dir);
    let first = event("same", 2, 2, "fail");
    duplicate_store.append(&first).unwrap();
    let row = fs::read(dir.store_path()).unwrap();
    OpenOptions::new()
        .append(true)
        .open(dir.store_path())
        .unwrap()
        .write_all(&row)
        .unwrap();
    assert_eq!(duplicate_store.query(&query()).unwrap().len(), 1);

    let conflicting = event("same", 3, 3, "error");
    assert!(
        duplicate_store
            .append(&conflicting)
            .unwrap_err()
            .contains("duplicate-conflict")
    );

    let second = event("second", 1, 1, "pass");
    duplicate_store.append(&second).unwrap();
    let bytes = fs::read(dir.store_path()).unwrap();
    let mut rows: Vec<&[u8]> = bytes.split_inclusive(|byte| *byte == b'\n').collect();
    rows.reverse();
    fs::write(dir.store_path(), rows.concat()).unwrap();
    let ids: Vec<_> = store(&dir)
        .query(&query())
        .unwrap()
        .iter()
        .map(|event| event.event_id().to_owned())
        .collect();
    assert_eq!(ids, ["second", "same"]);
}

#[test]
fn interrupted_partial_tail_is_visible_and_explicitly_recoverable() {
    let dir = TestDir::new("partial-recovery");
    let store = store(&dir);
    store.append(&event("valid", 1, 1, "fail")).unwrap();
    OpenOptions::new()
        .append(true)
        .open(dir.store_path())
        .unwrap()
        .write_all(b"{\"row_version\":\"SemanticEventRow-v1\"")
        .unwrap();
    assert!(
        store
            .query(&query())
            .unwrap_err()
            .contains("truncated-tail")
    );
    let removed = store.recover_truncated_tail().unwrap();
    assert!(removed > 0);
    assert_eq!(store.query(&query()).unwrap()[0].event_id(), "valid");
    assert_eq!(store.recover_truncated_tail().unwrap(), 0);
}

fn mutate_text(dir: &TestDir, change: impl FnOnce(String) -> String) {
    let text = fs::read_to_string(dir.store_path()).unwrap();
    fs::write(dir.store_path(), change(text)).unwrap();
}
