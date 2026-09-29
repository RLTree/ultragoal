#![cfg(unix)]

#[path = "cli_contract/state/authority_inputs.rs"]
mod authority_inputs;
#[path = "observability_public_diagnosis_contract/boundaries.rs"]
mod boundaries;
#[path = "observability_public_diagnosis_contract/concurrency.rs"]
mod concurrency;
#[path = "observability_public_diagnosis_contract/journeys.rs"]
mod journeys;
#[path = "observability_authority_fixture.rs"]
mod observability_authority_fixture;
#[path = "observability_fixture_scratch.rs"]
mod observability_fixture_scratch;
#[path = "observability_public_diagnosis_contract/recovery.rs"]
mod recovery;
#[path = "observability_public_diagnosis_contract/saturated_window.rs"]
mod saturated_window;
#[path = "observability_public_diagnosis_contract/scenario/mod.rs"]
mod scenario;

use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    id: String,
    class: String,
    expect: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalog {
    schema_version: String,
    temporary_root: String,
    source_id: String,
    public_schema: String,
    default_mode: String,
    external_export_default: String,
    claim_effect: String,
    cases: Vec<Case>,
}

#[test]
fn public_diagnosis_fixture_catalog_closes_required_classes() {
    let catalog: Catalog = serde_json::from_slice(
        &std::fs::read(
            scenario::live_root().join("fixtures/observability-public-diagnosis/cases.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        catalog.schema_version,
        "ObservabilityPublicDiagnosisFixtures-v1"
    );
    assert_eq!(catalog.temporary_root, scenario::BASE);
    assert_eq!(catalog.source_id, scenario::SOURCE_ID);
    assert_eq!(catalog.public_schema, "PublicCausalDiagnosis-v1");
    assert_eq!(catalog.default_mode, "local-only");
    assert_eq!(
        catalog.external_export_default,
        "disabled-safe-default-OD-004-OD-007"
    );
    assert_eq!(catalog.claim_effect, "none");
    let ids = catalog
        .cases
        .iter()
        .map(|case| case.id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(ids.len(), catalog.cases.len());
    let classes = catalog
        .cases
        .iter()
        .map(|case| case.class.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        classes,
        [
            "concurrency",
            "corruption",
            "false-pass",
            "fresh",
            "limit",
            "network",
            "path",
            "positive",
            "privacy",
            "race",
            "recovery",
            "repeat",
            "special-file",
        ]
        .into_iter()
        .collect()
    );
    assert!(catalog.cases.iter().all(|case| !case.expect.is_empty()));
}
