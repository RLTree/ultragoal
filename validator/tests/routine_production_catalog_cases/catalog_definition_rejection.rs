use super::*;

#[test]
pub(crate) fn unknown_duplicate_ambiguous_missing_and_behavior_substitutions_fail_closed() {
    let unknown = String::from_utf8(VALID_CATALOG.to_vec())
        .unwrap()
        .replacen("\"node_id\": \"verify\"", "\"node_id\": \"unknown\"", 1)
        .into_bytes();
    let root = TestRoot::new("unknown-definition", &unknown);
    assert_eq!(
        load_raw(&root, &unknown, full_adoption(&unknown, CANDIDATE_ID)),
        "catalog-definition-node-unknown"
    );

    let duplicate = String::from_utf8(VALID_CATALOG.to_vec())
        .unwrap()
        .replacen("\"node_id\": \"verify\"", "\"node_id\": \"syntax\"", 1)
        .into_bytes();
    let root = TestRoot::new("duplicate-definition", &duplicate);
    assert!(matches!(
        load_raw(&root, &duplicate, full_adoption(&duplicate, CANDIDATE_ID)),
        "catalog-definition-node-ambiguous" | "catalog-definition-node-duplicated"
    ));

    let ambiguous = String::from_utf8(VALID_CATALOG.to_vec())
        .unwrap()
        .replacen("\"node_id\": \"verify\"", "\"node_id\": \"Syntax\"", 1)
        .into_bytes();
    let root = TestRoot::new("ambiguous-definition", &ambiguous);
    assert_eq!(
        load_raw(&root, &ambiguous, full_adoption(&ambiguous, CANDIDATE_ID)),
        "catalog-definition-node-ambiguous"
    );

    let mut missing: serde_json::Value = serde_json::from_slice(VALID_CATALOG).unwrap();
    missing["routines"].as_array_mut().unwrap().pop();
    let missing = serde_json::to_vec_pretty(&missing).unwrap();
    let root = TestRoot::new("missing-definition", &missing);
    assert_eq!(
        load_raw(&root, &missing, full_adoption(&missing, CANDIDATE_ID)),
        "catalog-definition-set-inexact"
    );

    let mut behavior: serde_json::Value = serde_json::from_slice(VALID_CATALOG).unwrap();
    behavior["routines"][0]["behavior_id"] = serde_json::json!("shell-command-v1");
    let behavior = serde_json::to_vec_pretty(&behavior).unwrap();
    let root = TestRoot::new("behavior-substitution", &behavior);
    assert_eq!(
        load_raw(&root, &behavior, full_adoption(&behavior, CANDIDATE_ID)),
        "catalog-behavior-unsupported"
    );
}

#[cfg(unix)]
#[test]
pub(crate) fn definition_candidate_input_and_dependency_drift_after_parse_are_refused() {
    let root = TestRoot::new("definition-drift", VALID_CATALOG);
    let catalog = load_full(&root, CANDIDATE_ID);
    root.write_catalog(
        &String::from_utf8(VALID_CATALOG.to_vec())
            .unwrap()
            .replace("\"timeout_ms\": 60000", "\"timeout_ms\": 60001")
            .into_bytes(),
    );
    assert_eq!(
        catalog.verify_current().unwrap_err().code(),
        "catalog-sealed-file-changed"
    );

    let root = TestRoot::new("candidate-drift", VALID_CATALOG);
    let catalog = load_full(&root, CANDIDATE_ID);
    assert_eq!(
        catalog
            .bind_selected(request(&catalog, &root, OTHER_CANDIDATE_ID, false))
            .unwrap_err()
            .code(),
        "catalog-selection-binding-stale"
    );

    let root = TestRoot::new("input-drift", VALID_CATALOG);
    let catalog = load_full(&root, CANDIDATE_ID);
    let stale_selected = selected(&root, false);
    fs::write(root.path().join("tests/input.txt"), b"mutated after plan\n").unwrap();
    let request = CatalogSelectionRequest::new(
        catalog.catalog_id(),
        GRAPH_ID,
        CANDIDATE_ID,
        PLAN_ID,
        stale_selected,
        vec![runner(
            "ultragoal",
            TRUE_TOOL_ID,
            Path::new("/usr/bin/true"),
        )],
    )
    .unwrap();
    assert_eq!(
        catalog.bind_selected(request).unwrap_err().code(),
        "catalog-selection-transitive-input-stale"
    );

    let root = TestRoot::new("dependency-omission", VALID_CATALOG);
    let catalog = load_full(&root, CANDIDATE_ID);
    let verify_only = selected(&root, false).pop().unwrap();
    assert_eq!(
        CatalogSelectionRequest::new(
            catalog.catalog_id(),
            GRAPH_ID,
            CANDIDATE_ID,
            PLAN_ID,
            vec![verify_only],
            vec![runner(
                "ultragoal",
                TRUE_TOOL_ID,
                Path::new("/usr/bin/true")
            )],
        )
        .unwrap_err()
        .code(),
        "catalog-selection-dependency-closure-incomplete"
    );
}
