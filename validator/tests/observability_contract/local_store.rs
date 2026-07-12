use super::observability::{
    CausalExplanation, EventQuery, EventStore, ExportAdapter, SemanticEvent,
};
use super::support::{TestDir, event, query, store, tree_snapshot};

#[test]
fn public_contract_names_compile_from_the_owned_wrapper() {
    fn witness<T: ExportAdapter>(
        _: &SemanticEvent,
        _: &EventStore,
        _: &EventQuery,
        _: &CausalExplanation,
        _: &mut T,
    ) {
    }
    let _ = witness::<super::support::MockAdapter>;
}

#[test]
fn append_query_order_idempotence_and_explicit_cause_roundtrip() {
    let dir = TestDir::new("roundtrip");
    let store = store(&dir);
    let root = event("root", 20, 2, "fail");
    let mut child = event("child", 10, 9, "error");
    child.set_parent("root").unwrap();
    let earlier = event("earlier", 10, 1, "pass");

    assert!(store.append(&root).unwrap());
    assert!(store.append(&child).unwrap());
    assert!(store.append(&earlier).unwrap());
    assert!(
        !store.append(&root).unwrap(),
        "exact duplicate is idempotent"
    );

    let rows = store.query(&query()).unwrap();
    let ids: Vec<_> = rows.iter().map(SemanticEvent::event_id).collect();
    assert_eq!(ids, ["earlier", "child", "root"]);

    let explanation = store.explain(&query(), "child").unwrap();
    assert_eq!(explanation.classification(), "observed-cause");
    assert_eq!(explanation.causal_event_ids(), ["root", "child"]);
    assert!(!explanation.can_raise_claim());
    assert!(explanation.summary().contains("explicit"));
    assert!(explanation.repair().contains("rerun"));
}

#[test]
fn empty_and_missing_stores_are_safe_and_causality_is_withheld() {
    let dir = TestDir::new("missing");
    let store = store(&dir);
    assert!(store.query(&query()).unwrap().is_empty());
    assert!(
        !dir.store_path().exists(),
        "query must not create a missing store"
    );

    let explanation = store.explain(&query(), "missing").unwrap();
    assert_eq!(explanation.classification(), "missing-evidence");
    assert_eq!(
        explanation.diagnostic_code(),
        "observe-evidence-missing:store-empty"
    );
    assert!(
        !dir.store_path().exists(),
        "explain must not create a missing store"
    );
}

#[test]
fn recursive_before_after_snapshot_proves_query_and_explain_have_no_hidden_writes() {
    let dir = TestDir::new("read-purity");
    let store = store(&dir);
    store.append(&event("failed", 1, 1, "fail")).unwrap();
    let before = tree_snapshot(dir.path());

    let first = store.query(&query()).unwrap();
    let second = store.query(&query()).unwrap();
    let explanation = store.explain(&query(), "failed").unwrap();

    assert_eq!(first, second);
    assert_eq!(explanation.classification(), "observed-cause");
    assert_eq!(tree_snapshot(dir.path()), before);
}

#[test]
fn exact_filters_remain_bounded_and_deterministic() {
    let dir = TestDir::new("filters");
    let store = store(&dir);
    store.append(&event("pass-1", 1, 1, "pass")).unwrap();
    store.append(&event("fail-1", 2, 1, "fail")).unwrap();
    let query = query()
        .operation("check.run")
        .unwrap()
        .outcome("fail")
        .unwrap()
        .since(2)
        .until(2)
        .unwrap()
        .limit(1)
        .unwrap();
    assert_eq!(store.query(&query).unwrap()[0].event_id(), "fail-1");
}
