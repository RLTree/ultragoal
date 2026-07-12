use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureContract {
    schema: String,
    isolated_paths: Vec<String>,
    lifecycle_intents: Vec<String>,
    false_pass_inputs: Vec<String>,
    forbidden_claims: Vec<String>,
    claim_effect: String,
}

#[test]
fn fixture_contract_is_exact_and_cannot_be_read_as_host_or_runtime_proof() {
    let contract: FixtureContract = serde_json::from_str(include_str!(
        "../../../fixtures/plugin-distribution-adapter/scenarios.json"
    ))
    .unwrap();
    assert_eq!(
        contract.schema,
        "harness-ultragoal.plugin-distribution-adapter-fixture.v1"
    );
    assert_eq!(
        contract.isolated_paths,
        [
            "installed/harness-ultragoal.hugpkg",
            "cache/harness-ultragoal.hugpkg"
        ]
    );
    assert_eq!(contract.lifecycle_intents.len(), 8);
    assert_eq!(
        contract
            .lifecycle_intents
            .iter()
            .collect::<BTreeSet<_>>()
            .len(),
        8
    );
    assert_eq!(contract.false_pass_inputs.len(), 5);
    assert!(contract.forbidden_claims.contains(&"runtime".to_owned()));
    assert!(contract.forbidden_claims.contains(&"install".to_owned()));
    assert_eq!(contract.claim_effect, "none");
}
