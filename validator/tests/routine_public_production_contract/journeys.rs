use super::scenario::{Fixture, pass_node, prefix_route, tree};
use serde_json::Value;

#[test]
fn fixture_matrix_names_the_public_production_contract_without_claim_effect() {
    let value: Value = serde_json::from_slice(include_bytes!(
        "../../../fixtures/routine-public-production/cases.json"
    ))
    .unwrap();
    assert_eq!(value["schema_version"], "RoutinePublicProductionCases-v2");
    assert_eq!(value["supported_host"], "target_vendor=apple");
    assert_eq!(value["claim_effect"], "none");
    let cases = value["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 11);
    assert_eq!(
        cases
            .iter()
            .filter_map(Value::as_str)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        11
    );
    assert!(
        value["installed_runtime_ceiling"]
            .as_str()
            .unwrap()
            .contains("immutable installed ultragoal runtime")
    );
}

#[test]
fn clean_public_binary_is_noop_without_opening_host_authority() {
    let fixture = Fixture::new(
        "clean-noop",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        false,
        false,
    );
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    let before_status = fixture.status();
    let output = fixture.run();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    let value = Fixture::value(&output);
    assert_eq!(value["schema_version"], "RoutinePublicProductionOutcome-v1");
    assert_eq!(value["status"], "clean-no-op");
    assert_eq!(value["effect"], "none");
    assert_eq!(value["claim_effect"], "none");
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
    assert_eq!(fixture.status(), before_status);
}

#[test]
fn dirty_development_binary_refuses_before_host_or_workspace_authority() {
    let fixture = Fixture::new(
        "execute-reuse",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    let before_status = fixture.status();
    let output = fixture.run();
    assert_ne!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let diagnostic: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(
        diagnostic["diagnostic_id"],
        "successor_runtime_authority_required"
    );
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
    assert_eq!(fixture.status(), before_status);
}
