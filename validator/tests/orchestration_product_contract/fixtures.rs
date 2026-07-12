use serde_json::Value;
use std::collections::BTreeSet;

const CLEAN: &str = include_str!("../../../fixtures/orchestration-product/clean-start.json");
const INTERRUPTED: &str =
    include_str!("../../../fixtures/orchestration-product/interrupted-run.json");
const AMBIGUOUS: &str =
    include_str!("../../../fixtures/orchestration-product/ambiguous-recovery.json");
const CONCURRENT: &str =
    include_str!("../../../fixtures/orchestration-product/concurrent-reconcile.json");
const MUTATIONS: &str =
    include_str!("../../../fixtures/orchestration-product/mutation-controls.json");

#[test]
fn fixture_registry_is_exact_and_rejects_unknown_rows() {
    let values = [CLEAN, INTERRUPTED, AMBIGUOUS, CONCURRENT, MUTATIONS]
        .map(|bytes| serde_json::from_str::<Value>(bytes).unwrap());
    let cases = values
        .iter()
        .map(|value| {
            assert_eq!(value["schema_version"], "OrchestrationProductFixture-v1");
            value["case"].as_str().unwrap().to_owned()
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        cases,
        BTreeSet::from([
            "ambiguous-recovery".to_owned(),
            "clean-start".to_owned(),
            "concurrent-reconcile".to_owned(),
            "interrupted-run".to_owned(),
            "mutation-controls".to_owned(),
        ])
    );
    let controls = values[4]["controls"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect::<BTreeSet<_>>();
    assert_eq!(controls.len(), 17);
    assert!(controls.contains("receipt-without-behavior"));
    assert!(controls.contains("green-query-with-hidden-write"));
    assert!(controls.contains("recovery-with-orphaned-effect"));
    assert!(controls.contains("orphaned-recovery-premutation-zero-write"));
    assert!(controls.contains("concurrent-recovery-single-commit"));
    assert!(controls.contains("preview-commit-regular-root-substitution"));
    assert!(controls.contains("replay-with-stale-candidate"));
}

#[test]
fn fixture_expectations_name_behavior_not_receipt_proxies() {
    let clean: Value = serde_json::from_str(CLEAN).unwrap();
    let interrupted: Value = serde_json::from_str(INTERRUPTED).unwrap();
    let ambiguous: Value = serde_json::from_str(AMBIGUOUS).unwrap();
    let concurrent: Value = serde_json::from_str(CONCURRENT).unwrap();
    assert_eq!(clean["expected"]["writes"], 0);
    assert_eq!(interrupted["expected"]["journal_advance"], 1);
    assert_eq!(ambiguous["expected"]["error_code"], "HUL-ORCH-PROD-010");
    assert_eq!(concurrent["expected"]["authoritative_outcomes"], 1);
    assert_eq!(concurrent["expected"]["orphaned_effects"], 0);
}
