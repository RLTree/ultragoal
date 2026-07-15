use super::scenario::{Fixture, pass_node, prefix_route, tree};
use serde_json::Value;
use std::fs;

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
    assert_eq!(cases.len(), 7);
    assert_eq!(
        cases
            .iter()
            .filter_map(Value::as_str)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        7
    );
    let runtime_cases = value["immutable_runtime_cases"].as_array().unwrap();
    assert_eq!(runtime_cases.len(), 8);
    assert_eq!(
        runtime_cases
            .iter()
            .filter_map(Value::as_str)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        8
    );
    assert!(
        value["installed_runtime_ceiling"]
            .as_str()
            .unwrap()
            .contains("immutable installed ultragoal runtime")
    );
}

#[test]
fn clean_public_routine_is_a_zero_effect_noop() {
    let mut fixture = Fixture::new(
        "clean-no-op",
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
    assert_eq!(value["status"], "clean-no-op");
    assert_eq!(value["effect"], "none");
    let after_root = tree(&fixture.root);
    for (path, value) in before_root {
        assert_eq!(
            after_root.get(&path),
            Some(&value),
            "source changed: {path}"
        );
    }
    assert_eq!(tree(&fixture.home), before_home);
    assert_eq!(fixture.status(), before_status);
    fixture.teardown_after_assertions();
}

#[test]
fn dirty_public_effect_executes_through_the_local_issuer_and_reuses() {
    let mut fixture = Fixture::new(
        "execute-reuse",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let before_root = tree(&fixture.root);
    let executed = fixture.run();
    assert_eq!(executed.status.code(), Some(0), "{executed:?}");
    assert!(executed.stderr.is_empty(), "{executed:?}");
    let executed_value = Fixture::value(&executed);
    assert_eq!(executed_value["status"], "executed");
    assert_eq!(executed_value["nodes"][0]["disposition"], "executed");
    let after_root = tree(&fixture.root);
    for (path, value) in before_root {
        assert_eq!(
            after_root.get(&path),
            Some(&value),
            "source changed: {path}"
        );
    }

    let reused = fixture.run();
    assert_eq!(reused.status.code(), Some(0), "{reused:?}");
    assert!(reused.stderr.is_empty(), "{reused:?}");
    let reused_value = Fixture::value(&reused);
    assert_eq!(reused_value["status"], "reused");
    assert_eq!(reused_value["nodes"][0]["disposition"], "reused");
    fixture.teardown_after_assertions();
}

#[test]
fn authorized_fresh_execution_then_exact_repeat_reuses_without_output_attribution() {
    let mut fixture = Fixture::new(
        "authorized-fresh-repeat",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    assert!(!fixture.root.join("target").exists());

    let executed = fixture.run();
    assert_eq!(executed.status.code(), Some(0), "{executed:?}");
    assert!(executed.stderr.is_empty(), "{executed:?}");
    let executed = Fixture::value(&executed);
    assert_eq!(executed["status"], "executed");
    assert_eq!(executed["nodes"][0]["disposition"], "executed");
    let scope = fixture.root.join("target/routine/compile");
    assert!(scope.is_dir());
    assert_eq!(fs::read_dir(&scope).unwrap().count(), 0);

    let reused = fixture.run();
    assert_eq!(reused.status.code(), Some(0), "{reused:?}");
    assert!(reused.stderr.is_empty(), "{reused:?}");
    let reused = Fixture::value(&reused);
    assert_eq!(reused["status"], "reused");
    assert_eq!(reused["nodes"][0]["disposition"], "reused");
    assert_eq!(fs::read_dir(scope).unwrap().count(), 0);
    fixture.teardown_after_assertions();
}

#[test]
fn cache_binding_misses_across_targets_and_changes_then_reuses_exact_repeat() {
    let mut first = Fixture::new(
        "binding-first",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let mut second = Fixture::new(
        "binding-second",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        false,
    );
    second.home.clone_from(&first.home);

    assert_status(&first, "executed");
    assert_status(&first, "reused");
    assert_status(&second, "executed");
    assert_status(&second, "reused");
    assert_status(&first, "executed");
    assert_status(&first, "reused");

    fs::write(
        first.root.join("src/lib.rs"),
        b"pub fn value() -> u8 { 3 }\n",
    )
    .unwrap();
    assert_status(&first, "executed");
    assert_status(&first, "reused");
    second.teardown_after_assertions();
    first.teardown_after_assertions();
}

fn assert_status(fixture: &Fixture, expected: &str) {
    let output = fixture.run();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    assert_eq!(Fixture::value(&output)["status"], expected);
}
