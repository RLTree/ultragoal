use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalog {
    schema_version: String,
    claim_effect: String,
    no_claim_statement: String,
    cases: Vec<JourneyCase>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct JourneyCase {
    id: String,
    expected_mode: String,
    expected_reuse: String,
    preserves_unrelated_work: bool,
}

#[test]
fn journey_catalog_is_exact_non_claiming_and_complete() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/routine-recovery-reuse/cases.json");
    let bytes = std::fs::read(path).unwrap();
    let catalog: Catalog = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        catalog.schema_version,
        "RoutineRecoveryReuseJourneyCases-v1"
    );
    assert_eq!(catalog.claim_effect, "none");
    assert!(catalog.no_claim_statement.contains("do not claim"));
    let expected = BTreeSet::from([
        "clean-no-op",
        "conflict-expands-strict",
        "dirty-dependency-closed",
        "exact-repeat-reuse",
        "mutation-forces-miss",
        "optional-accelerator-fallback",
    ]);
    assert_eq!(
        catalog
            .cases
            .iter()
            .map(|case| case.id.as_str())
            .collect::<BTreeSet<_>>(),
        expected
    );
    assert!(catalog.cases.iter().all(|case| {
        matches!(case.expected_mode.as_str(), "no-op" | "fast" | "strict")
            && !case.expected_reuse.is_empty()
            && case.preserves_unrelated_work
    }));
}
