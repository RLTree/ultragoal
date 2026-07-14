use super::*;

#[test]
pub(crate) fn unknown_duplicate_ambiguous_and_missing_definitions_fail_closed() {
    let case_aliased_fallback = AdoptedRoutineNode::new(
        "verify",
        vec!["syntax".to_owned()],
        "true",
        Some("TRUE".to_owned()),
    )
    .unwrap_err();
    assert_eq!(
        case_aliased_fallback.code(),
        "catalog-adoption-runner-duplicated"
    );

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

    let mut conflicting_runner_authority: serde_json::Value =
        serde_json::from_slice(VALID_CATALOG).unwrap();
    conflicting_runner_authority["routines"][1]["primary"]["program_sha256"] =
        serde_json::json!(FALSE_PROGRAM_SHA256);
    let conflicting_runner_authority =
        serde_json::to_vec_pretty(&conflicting_runner_authority).unwrap();
    let root = TestRoot::new(
        "conflicting-runner-authority",
        &conflicting_runner_authority,
    );
    assert_eq!(
        load_raw(
            &root,
            &conflicting_runner_authority,
            full_adoption(&conflicting_runner_authority, CANDIDATE_ID),
        ),
        "catalog-runner-authority-ambiguous"
    );

    let mut case_alias_same_authority: serde_json::Value =
        serde_json::from_slice(VALID_CATALOG).unwrap();
    case_alias_same_authority["routines"][1]["primary"]["tool"] = serde_json::json!("TRUE");
    let case_alias_same_authority = serde_json::to_vec_pretty(&case_alias_same_authority).unwrap();
    let case_alias_adoption = |bytes: &[u8]| {
        CatalogAdoption::new(
            sha(bytes),
            bytes.len() as u64,
            GRAPH_ID,
            CANDIDATE_ID,
            vec![
                AdoptedRoutineNode::new("syntax", Vec::<String>::new(), "true", None).unwrap(),
                AdoptedRoutineNode::new(
                    "verify",
                    vec!["syntax".to_owned()],
                    "TRUE",
                    Some("false".to_owned()),
                )
                .unwrap(),
            ],
        )
        .unwrap()
    };
    let root = TestRoot::new("case-alias-same-authority", &case_alias_same_authority);
    assert_eq!(
        load_raw(
            &root,
            &case_alias_same_authority,
            case_alias_adoption(&case_alias_same_authority),
        ),
        "catalog-runner-spelling-ambiguous"
    );

    let mut case_alias_conflicting_authority: serde_json::Value =
        serde_json::from_slice(&case_alias_same_authority).unwrap();
    case_alias_conflicting_authority["routines"][1]["primary"]["program_sha256"] =
        serde_json::json!(FALSE_PROGRAM_SHA256);
    let case_alias_conflicting_authority =
        serde_json::to_vec_pretty(&case_alias_conflicting_authority).unwrap();
    let root = TestRoot::new(
        "case-alias-conflicting-authority",
        &case_alias_conflicting_authority,
    );
    assert_eq!(
        load_raw(
            &root,
            &case_alias_conflicting_authority,
            case_alias_adoption(&case_alias_conflicting_authority),
        ),
        "catalog-runner-spelling-ambiguous"
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

    let root = TestRoot::new("runner-expectation-drift", VALID_CATALOG);
    let catalog = load_full(&root, CANDIDATE_ID);
    root.write_catalog(
        &String::from_utf8(VALID_CATALOG.to_vec())
            .unwrap()
            .replacen(TRUE_PROGRAM_SHA256, FALSE_PROGRAM_SHA256, 1)
            .into_bytes(),
    );
    assert_eq!(
        catalog.verify_current().unwrap_err().code(),
        "catalog-sealed-file-changed"
    );

    let root = TestRoot::new("candidate-drift", VALID_CATALOG);
    let catalog = load_full(&root, CANDIDATE_ID);
    let error = catalog
        .bind_selected(request(&catalog, &root, OTHER_CANDIDATE_ID, false))
        .unwrap_err();
    assert_eq!(error.code(), "catalog-selection-binding-stale");

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
        vec![runner("true", TRUE_TOOL_ID, Path::new("/usr/bin/true"))],
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
            vec![runner("true", TRUE_TOOL_ID, Path::new("/usr/bin/true"))],
        )
        .unwrap_err()
        .code(),
        "catalog-selection-dependency-closure-incomplete"
    );
}
