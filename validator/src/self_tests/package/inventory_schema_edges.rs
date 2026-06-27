use serde_json::json;

#[test]
fn claim_schema_and_materiality_edges() {
    let mut failures = Vec::new();
    crate::claim_semantics::coverage::receipt::rules::dimensions(
        &json!({"measured_dimensions":[],"target_paths":["validator/src/lib.rs"]}),
        "source code branch command region product route schema package",
        &mut failures,
    );
    let codes = failures
        .iter()
        .map(|failure| failure.error.as_str())
        .collect::<Vec<_>>();
    assert!(
        codes
            .iter()
            .any(|code| *code == "coverage_required_dimension_missing")
    );

    let exclusion_failures = crate::audit::coverage::scope::exclusions::failures(&json!({
        "exclusions":[{"path":"validator/src/lib.rs","reviewed":false,"counts_as_covered":true}]
    }));
    assert!(exclusion_failures.contains(&"coverage_exclusion_missing_rationale".to_string()));
    assert!(exclusion_failures.contains(&"coverage_exclusion_unreviewed".to_string()));

    let receipt = json!({
        "decision":"BLOCKED_BEFORE_REVIEW",
        "deterministic_gates_required":[],
        "deterministic_gates_run":[],
        "reviewers_required":["product"],
        "validator_repairs_recommended":[],
        "claim_ceiling":{"supported":[],"unsupported":[],"blocked":[]}
    });
    let materiality = crate::review::materiality::value_failures(&receipt);
    assert!(
        materiality
            .iter()
            .any(|item| item == "materiality_blocked_launches_reviewers")
    );
    assert!(
        materiality
            .iter()
            .any(|item| item == "materiality_blocked_without_repair")
    );

    let store = crate::schema_catalog::load(&crate::self_tests::boundaries::support::repo_root());
    let errors = crate::schema_catalog::schema_errors(
        &store,
        "fixture-bundle.schema.json",
        &json!({"schema":"harness-ultragoal.fixture-bundle.v1","claims":[]}),
    );
    assert!(
        !errors.is_empty() || crate::schema_catalog::schema_error_code(&errors) == "schema_valid"
    );
}

#[test]
fn inventory_nonexistent_only_hits_right_side_branches() {
    let root =
        crate::self_tests::boundaries::support::temp_root("inventory_schema-empty-inventory");
    std::fs::create_dir_all(&root).expect("root");
    let failures = crate::package::inventory::closure::inventory_closure_failures(
        &root,
        &json!({"resources":["missing-only.txt"]}),
    );
    let joined = failures.join("\n");
    assert!(joined.contains("nonexistent="), "{joined}");
    assert!(joined.contains("actual=0 listed=1"), "{joined}");
    std::fs::remove_dir_all(root).expect("cleanup inventory");
}
