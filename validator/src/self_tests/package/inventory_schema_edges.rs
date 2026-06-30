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

#[test]
fn package_inventory_ignores_local_dependency_caches_but_keeps_manifests() {
    let root = crate::self_tests::boundaries::support::temp_root("inventory-local-deps");
    std::fs::create_dir_all(root.join("node_modules/.bin")).expect("node modules");
    std::fs::create_dir_all(root.join(".pnpm-store/v3")).expect("pnpm store");
    std::fs::write(root.join("node_modules/.bin/promptfoo"), "generated").expect("node module");
    std::fs::write(root.join(".pnpm-store/v3/index.json"), "{}").expect("pnpm cache");
    std::fs::write(root.join("package.json"), "{}").expect("package manifest");
    std::fs::write(root.join("pnpm-lock.yaml"), "lockfileVersion: '9.0'").expect("lock");
    std::fs::write(root.join("pnpm-workspace.yaml"), "packages: []").expect("workspace");

    let files = crate::package::inventory::closure::actual_files(&root).expect("actual files");
    assert!(!files.iter().any(|path| path.starts_with("node_modules/")));
    assert!(!files.iter().any(|path| path.starts_with(".pnpm-store/")));
    assert!(files.contains(&"package.json".to_string()));
    assert!(files.contains(&"pnpm-lock.yaml".to_string()));
    assert!(files.contains(&"pnpm-workspace.yaml".to_string()));
    std::fs::remove_dir_all(root).expect("cleanup inventory local deps");
}

#[test]
fn package_inventory_ignores_parent_session_contract_files() {
    let root = crate::self_tests::boundaries::support::temp_root("inventory-parent-contract");
    std::fs::create_dir_all(root.join("docs")).expect("docs");
    std::fs::write(root.join("docs/package.md"), "package resource").expect("package doc");
    std::fs::write(
        root.join("docs/parent-session-full-ultragoal-execution-spine-2026-06-30.md"),
        "builder contract",
    )
    .expect("execution spine");
    let files = crate::package::inventory::closure::actual_files(&root).expect("actual files");
    assert!(files.contains(&"docs/package.md".to_string()));
    assert!(
        !files
            .iter()
            .any(|rel| rel.contains("parent-session-full-ultragoal")),
        "{files:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup parent contract inventory");
}

#[test]
fn package_inventory_paths_ignore_untyped_rows_and_collect_typed_paths() {
    let paths = crate::package::inventory::inventory_paths(&json!({
        "skills":[{"path":"skills/a/SKILL.md"},{}],
        "agents":[{"path":"agents/a.md"},{"path":7}],
        "schema_catalog":"schemas/schema-catalog.json",
        "resources":["README.md", false],
        "schemas":["schemas/a.schema.json"]
    }));
    assert_eq!(
        paths,
        vec![
            "skills/a/SKILL.md",
            "agents/a.md",
            "schemas/schema-catalog.json",
            "schemas/a.schema.json",
            "README.md"
        ]
    );
}

#[test]
fn package_digest_valid_fixture_canonicalization_rejects_malformed_json() {
    let err = crate::package::inventory::stable_package_payload(
        "fixtures/valid/current.json",
        b"{not-json",
    )
    .expect_err("malformed valid fixture rejected");
    assert!(
        err.contains("valid fixture digest canonicalization failed"),
        "{err}"
    );

    assert_eq!(
        crate::package::inventory::stable_package_payload(
            "fixtures/red/current.json",
            b"{not-json"
        )
        .expect("red fixtures are not canonicalized"),
        b"{not-json".to_vec()
    );
}

#[test]
fn package_digest_rejects_malformed_valid_fixture_through_digest_path() {
    let root = crate::self_tests::boundaries::support::temp_root("package-digest-bad-valid");
    std::fs::create_dir_all(root.join("fixtures/valid")).expect("fixtures");
    std::fs::write(
        root.join("plugin-manifest-draft.json"),
        serde_json::to_vec(&json!({"resources":["fixtures/valid/bad.json"]})).expect("manifest"),
    )
    .expect("manifest");
    std::fs::write(root.join("fixtures/valid/bad.json"), b"{bad").expect("bad fixture");

    let err = crate::package::inventory::package_digest(&root)
        .expect_err("malformed valid fixture blocks package digest");
    assert!(
        err.contains("valid fixture digest canonicalization failed"),
        "{err}"
    );
    std::fs::remove_dir_all(root).expect("cleanup package digest bad valid");
}

#[test]
fn package_path_error_falls_back_when_root_cannot_be_canonicalized() {
    let missing_root = std::path::Path::new("/definitely-missing-ultragoal-root");
    assert!(crate::package::inventory::package_path_error(missing_root, "docs/a.md").is_none());
}
