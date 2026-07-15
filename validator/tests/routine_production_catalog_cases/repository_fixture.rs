use super::*;

pub(crate) fn commit_fixture(root: &Path) {
    git(root, &["init", "-q"]);
    git(
        root,
        &["config", "user.email", "routine-catalog@example.invalid"],
    );
    git(root, &["config", "user.name", "Routine Catalog Contract"]);
    git(root, &["add", "."]);
    git(root, &["commit", "-q", "-m", "fixture"]);
}

pub(crate) fn status(root: &Path) -> Vec<u8> {
    git(
        root,
        &[
            "--no-optional-locks",
            "status",
            "--porcelain=v2",
            "-z",
            "--untracked-files=all",
        ],
    )
}

pub(crate) fn one_node_catalog(read_sources: &[String], output_scopes: &[String]) -> Vec<u8> {
    serde_json::to_vec_pretty(&serde_json::json!({
        "schema_version": "RoutineProductionCatalog-v2",
        "graph_id": GRAPH_ID,
        "routines": [{
            "node_id": "syntax",
            "behavior_id": "rust-source-syntax-v1",
            "depends_on": [],
            "read_sources": read_sources,
            "timeout_ms": 60000,
            "output_budget_bytes": 4194304,
            "output_scopes": output_scopes
        }]
    }))
    .unwrap()
}

#[test]
pub(crate) fn fixture_catalog_declares_the_exact_adversarial_matrix_without_claim_effect() {
    let cases: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../fixtures/routine-production-catalog/cases.json"
    ))
    .unwrap();
    assert_eq!(cases["schema_version"], "RoutineProductionCatalogCases-v1");
    assert_eq!(cases["claim_effect"], "none");
    assert_eq!(cases["cases"].as_array().unwrap().len(), 16);
    let ids = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["id"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(ids.len(), 16);
}

#[cfg(unix)]
#[test]
pub(crate) fn valid_catalog_binds_exact_primary_invocations_deterministically_and_without_writes() {
    let mut root = TestRoot::new("valid-primary", VALID_CATALOG);
    let before = tree(root.path());
    let first_catalog = load_full(&root, CANDIDATE_ID);
    assert_eq!(first_catalog.definition_count(), 2);
    assert_eq!(
        first_catalog.definition_ids().collect::<Vec<_>>(),
        ["routine.syntax.v2", "routine.verify.v2"]
    );
    let first = first_catalog
        .bind_selected(request(&first_catalog, &root, CANDIDATE_ID, false))
        .unwrap();
    let second_catalog = load_full(&root, CANDIDATE_ID);
    let second = second_catalog
        .bind_selected(request(&second_catalog, &root, CANDIDATE_ID, false))
        .unwrap();
    assert_eq!(first.invocation_set_id(), second.invocation_set_id());
    assert_eq!(first, second);
    assert_eq!(
        first
            .invocations()
            .iter()
            .map(|row| row.node_id())
            .collect::<Vec<_>>(),
        ["syntax", "verify"]
    );
    assert!(first.invocations().iter().all(|row| {
        row.selected_tool() == "ultragoal"
            && row.behavior_id() == "rust-source-syntax-v1"
            && row.environment()["LANG"] == "C"
            && row.environment()["LC_ALL"] == "C"
            && row.environment()["PATH"] == "/usr/bin"
            && row.arguments() == ["--json", "check", "routine"]
            && !row.read_sources().is_empty()
            && row.timeout_ms() > 0
            && row.output_budget_bytes() > 0
            && row
                .output_scopes()
                .iter()
                .all(|scope| scope.relative_path().starts_with("target/"))
    }));
    assert_eq!(tree(root.path()), before);
    drop(second_catalog);
    drop(first_catalog);
    root.teardown_after_assertions();
}

#[cfg(unix)]
#[test]
pub(crate) fn repository_catalog_cannot_choose_a_fallback() {
    let mut catalog: serde_json::Value = serde_json::from_slice(VALID_CATALOG).unwrap();
    catalog["routines"][0]["fallback"] = serde_json::json!({"tool": "false"});
    let bytes = serde_json::to_vec_pretty(&catalog).unwrap();
    let mut root = TestRoot::new("fallback-field", &bytes);
    assert_eq!(
        load_raw(&root, &bytes, full_adoption(&bytes, CANDIDATE_ID)),
        "catalog-source-invalid-json"
    );
    root.teardown_after_assertions();
}

#[cfg(unix)]
#[test]
pub(crate) fn exact_same_spelling_same_authority_reuse_selects_and_binds_both_definitions() {
    let mut root = TestRoot::new("exact-runner-reuse", VALID_CATALOG);
    let before = tree(root.path());
    let catalog = load_full(&root, CANDIDATE_ID);
    let bound = catalog
        .bind_selected(request(&catalog, &root, CANDIDATE_ID, false))
        .unwrap();

    assert_eq!(bound.invocations().len(), 2);
    assert!(
        bound
            .invocations()
            .iter()
            .all(|invocation| invocation.selected_tool() == "ultragoal")
    );
    assert_eq!(tree(root.path()), before);
    drop(catalog);
    root.teardown_after_assertions();
}
