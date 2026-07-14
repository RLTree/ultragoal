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

pub(crate) fn one_node_catalog(
    arguments: &[String],
    environment: &[(String, String)],
    read_sources: &[String],
    output_scopes: &[String],
    working_directory: &str,
) -> Vec<u8> {
    let environment = environment.iter().cloned().collect::<BTreeMap<_, _>>();
    serde_json::to_vec_pretty(&serde_json::json!({
        "schema_version": "RoutineProductionCatalog-v1",
        "graph_id": GRAPH_ID,
        "routines": [{
            "definition_id": "routine.syntax.v1",
            "node_id": "syntax",
            "depends_on": [],
            "working_directory": working_directory,
            "runner_policy": "immutable-single-process-exact-executable-v1",
            "read_policy": "selected-transitive-exact-regular-files-v1",
            "read_sources": read_sources,
            "environment": environment,
            "timeout_ms": 60000,
            "output_budget_bytes": 4194304,
            "output_scopes": output_scopes,
            "primary": {
                "tool": "true",
                "tool_identity_sha256": TRUE_TOOL_ID,
                "executable_path": "/usr/bin/true",
                "program_sha256": TRUE_PROGRAM_SHA256,
                "program_byte_length": SYSTEM_PROGRAM_BYTE_LENGTH,
                "program_unix_mode": SYSTEM_PROGRAM_UNIX_MODE,
                "arguments": arguments
            }
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
    let root = TestRoot::new("valid-primary", VALID_CATALOG);
    let before = tree(root.path());
    let first_catalog = load_full(&root, CANDIDATE_ID);
    assert_eq!(first_catalog.definition_count(), 2);
    assert_eq!(
        first_catalog.definition_ids().collect::<Vec<_>>(),
        ["routine.syntax.v1", "routine.verify.v1"]
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
        row.selected_tool() == "true"
            && row.environment()["LANG"] == "C"
            && row.environment()["LC_ALL"] == "C"
            && row.environment()["PATH"] == "/usr/bin"
            && row.arguments() == ["--version"]
            && !row.read_sources().is_empty()
            && row.timeout_ms() > 0
            && row.output_budget_bytes() > 0
            && row
                .output_scopes()
                .iter()
                .all(|scope| scope.relative_path().starts_with("target/"))
    }));
    assert_eq!(tree(root.path()), before);
}

#[cfg(unix)]
#[test]
pub(crate) fn conservative_fallback_requires_the_adopted_equivalent_recipe_and_exact_runner() {
    let root = TestRoot::new("valid-fallback", VALID_CATALOG);
    let catalog = load_full(&root, CANDIDATE_ID);
    let bound = catalog
        .bind_selected(request(&catalog, &root, CANDIDATE_ID, true))
        .unwrap();
    assert_eq!(bound.invocations()[0].selected_tool(), "true");
    assert_eq!(bound.invocations()[1].selected_tool(), "false");
}

#[cfg(unix)]
#[test]
pub(crate) fn exact_same_spelling_same_authority_reuse_selects_and_binds_both_definitions() {
    let root = TestRoot::new("exact-runner-reuse", VALID_CATALOG);
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
            .all(|invocation| invocation.selected_tool() == "true")
    );
    assert_eq!(tree(root.path()), before);
}
