use super::scenario::{Fixture, NodeSpec, pass_node, prefix_route, tree, wait_for_started};
use serde_json::Value;
use std::fs;
use std::time::Duration;

#[test]
fn fixture_matrix_names_the_public_production_contract_without_claim_effect() {
    let value: Value = serde_json::from_slice(include_bytes!(
        "../../../fixtures/routine-public-production/cases.json"
    ))
    .unwrap();
    assert_eq!(value["schema_version"], "RoutinePublicProductionCases-v1");
    assert_eq!(value["supported_host"], "target_vendor=apple");
    assert_eq!(value["claim_effect"], "none");
    let cases = value["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 20);
    assert_eq!(
        cases
            .iter()
            .filter_map(Value::as_str)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        20
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
fn dirty_public_binary_executes_then_authenticates_exact_durable_reuse() {
    let fixture = Fixture::new(
        "execute-reuse",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let first = fixture.run();
    assert_eq!(first.status.code(), Some(0), "{first:?}");
    assert!(first.stderr.is_empty(), "{first:?}");
    let first_value = Fixture::value(&first);
    assert_eq!(first_value["status"], "executed");
    assert_eq!(first_value["effect"], "workspace_write");
    assert_eq!(first_value["nodes"][0]["node_id"], "compile");
    assert_eq!(first_value["nodes"][0]["disposition"], "executed");
    assert_eq!(
        fs::read(fixture.root.join("target/routine/compile/result.txt")).unwrap(),
        b"compile"
    );
    assert!(fixture.cache_path().is_file());
    assert_eq!(fs::read_dir(fixture.authority_root()).unwrap().count(), 3);

    let output_before = fs::read(fixture.root.join("target/routine/compile/result.txt")).unwrap();
    let second = fixture.run();
    assert_eq!(second.status.code(), Some(0), "{second:?}");
    assert!(second.stderr.is_empty(), "{second:?}");
    let second_value = Fixture::value(&second);
    assert_eq!(second_value["status"], "reused");
    assert_eq!(second_value["nodes"][0]["disposition"], "reused");
    assert_eq!(
        fs::read(fixture.root.join("target/routine/compile/result.txt")).unwrap(),
        output_before
    );
}

#[test]
fn affected_selection_is_conservative_but_does_not_run_unrelated_nodes() {
    let nodes = [pass_node("compile", &[]), pass_node("docs", &[])];
    let routes = [
        prefix_route("route-src", "src", &["compile"]),
        prefix_route("route-docs", "docs", &["docs"]),
    ];
    let fixture = Fixture::new("partial-selection", &nodes, &routes, true, true);
    let output = fixture.run();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let value = Fixture::value(&output);
    assert_eq!(value["status"], "executed");
    assert_eq!(value["nodes"].as_array().unwrap().len(), 1);
    assert_eq!(value["nodes"][0]["node_id"], "compile");
    assert!(
        fixture
            .root
            .join("target/routine/compile/result.txt")
            .exists()
    );
    assert!(!fixture.root.join("target/routine/docs/result.txt").exists());
}

#[test]
fn dependency_closed_partial_failure_reports_stable_cause_and_withholds_cache() {
    let nodes = [
        pass_node("compile", &[]),
        NodeSpec {
            id: "verify",
            dependencies: &["compile"],
            primary: "bash",
            fallback: None,
            action: "fail",
            delay_seconds: 0,
            read_sources: &["src/lib.rs"],
        },
    ];
    let fixture = Fixture::new(
        "partial-failure",
        &nodes,
        &[prefix_route("route-src", "src", &["verify"])],
        true,
        true,
    );
    let output = fixture.run();
    assert_ne!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    let value = Fixture::value(&output);
    assert_eq!(value["status"], "incomplete");
    assert_eq!(value["nodes"].as_array().unwrap().len(), 2);
    assert_eq!(value["nodes"][0]["node_id"], "compile");
    assert_eq!(value["nodes"][0]["disposition"], "executed");
    assert_eq!(value["nodes"][1]["node_id"], "verify");
    assert_eq!(value["nodes"][1]["disposition"], "failed");
    let failure = value["nodes"][1]["failure_code"].as_str().unwrap();
    assert!(failure.starts_with("MEDIATOR-"), "{failure}");
    assert!(!failure.contains(fixture.root.to_str().unwrap()));
    assert!(!fixture.cache_path().exists());
}

#[test]
fn unavailable_primary_uses_only_the_adopted_fixed_template_fallback() {
    let node = NodeSpec {
        id: "compile",
        dependencies: &[],
        primary: "routine-unavailable",
        fallback: Some("bash"),
        action: "pass",
        delay_seconds: 0,
        read_sources: &["src/lib.rs"],
    };
    let fixture = Fixture::new(
        "fallback",
        &[node],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let output = fixture.run();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let value = Fixture::value(&output);
    assert_eq!(value["status"], "executed");
    assert_eq!(value["fallback_tool_count"], 1);
}

#[test]
fn interrupted_process_restarts_through_exact_durable_recovery() {
    let node = NodeSpec {
        id: "compile",
        dependencies: &[],
        primary: "bash",
        fallback: None,
        action: "pass",
        delay_seconds: 2,
        read_sources: &["src/lib.rs"],
    };
    let fixture = Fixture::new(
        "restart-recovery",
        &[node],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let mut child = fixture.spawn();
    wait_for_started(&fixture.authority_root());
    child.kill().unwrap();
    let _ = child.wait();
    std::thread::sleep(Duration::from_secs(3));
    let recovered = fixture.run();
    assert_eq!(recovered.status.code(), Some(0), "{recovered:?}");
    let value = Fixture::value(&recovered);
    assert_eq!(value["status"], "executed");
    assert_eq!(value["recovery_required"], false);
    assert!(fixture.cache_path().is_file());
}

#[test]
fn concurrent_public_processes_serialize_to_one_execution_and_one_reuse() {
    let node = NodeSpec {
        id: "compile",
        dependencies: &[],
        primary: "bash",
        fallback: None,
        action: "pass",
        delay_seconds: 1,
        read_sources: &["src/lib.rs"],
    };
    let fixture = Fixture::new(
        "concurrent",
        &[node],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let first = fixture.spawn();
    std::thread::sleep(Duration::from_millis(50));
    let second = fixture.spawn();
    let first = first.wait_with_output().unwrap();
    let second = second.wait_with_output().unwrap();
    assert_eq!(first.status.code(), Some(0), "{first:?}");
    assert_eq!(second.status.code(), Some(0), "{second:?}");
    let statuses = [Fixture::value(&first), Fixture::value(&second)]
        .into_iter()
        .map(|value| value["status"].as_str().unwrap().to_owned())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        statuses,
        std::collections::BTreeSet::from(["executed".to_owned(), "reused".to_owned()])
    );
}
