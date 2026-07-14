use super::*;

#[test]
pub(crate) fn execution_authority_fields_paths_and_prose_substitution_fail_closed() {
    for (field, value) in [
        ("working_directory", serde_json::json!(".")),
        ("environment", serde_json::json!({"PATH": "/tmp"})),
        ("arguments", serde_json::json!(["-c", "true"])),
        ("primary", serde_json::json!({"tool": "sh"})),
        ("fallback", serde_json::json!({"tool": "false"})),
    ] {
        let mut catalog: serde_json::Value = serde_json::from_slice(VALID_CATALOG).unwrap();
        catalog["routines"][0][field] = value;
        let bytes = serde_json::to_vec(&catalog).unwrap();
        let root = TestRoot::new(field, &bytes);
        assert_eq!(
            load_raw(&root, &bytes, full_adoption(&bytes, CANDIDATE_ID)),
            "catalog-source-invalid-json"
        );
    }

    let bytes = one_node_catalog(&["target/input.txt".to_owned()], &["target".to_owned()]);
    let root = TestRoot::new("overlapping-output", &bytes);
    assert_eq!(
        load_raw(&root, &bytes, one_node_adoption(&bytes)),
        "catalog-output-scope-overlaps-input"
    );

    let bytes = one_node_catalog(
        &["../secret.txt".to_owned()],
        &["target/routine-syntax".to_owned()],
    );
    let root = TestRoot::new("read-path-escape", &bytes);
    assert_eq!(
        load_raw(&root, &bytes, one_node_adoption(&bytes)),
        "catalog-repository-relative-path-required"
    );

    let mut prose: serde_json::Value = serde_json::from_slice(VALID_CATALOG).unwrap();
    prose
        .as_object_mut()
        .unwrap()
        .insert("proof_receipt".to_owned(), serde_json::json!("PASS"));
    let prose = serde_json::to_vec_pretty(&prose).unwrap();
    let root = TestRoot::new("prose-substitution", &prose);
    assert_eq!(
        load_raw(&root, &prose, full_adoption(&prose, CANDIDATE_ID)),
        "catalog-source-invalid-json"
    );

    let digest_only = serde_json::to_vec_pretty(&serde_json::json!({
        "schema_version": "RoutineProductionCatalog-v1",
        "graph_id": GRAPH_ID,
        "digest": sha(b"definition prose"),
        "routines": []
    }))
    .unwrap();
    let root = TestRoot::new("digest-only-substitution", &digest_only);
    assert_eq!(
        load_raw(&root, &digest_only, one_node_adoption(&digest_only)),
        "catalog-source-invalid-json"
    );
}

#[test]
pub(crate) fn retired_v1_catalog_has_no_behavioral_reader() {
    let mut legacy: serde_json::Value = serde_json::from_slice(VALID_CATALOG).unwrap();
    legacy["schema_version"] = serde_json::json!("RoutineProductionCatalog-v1");
    let legacy = serde_json::to_vec_pretty(&legacy).unwrap();
    let root = TestRoot::new("retired-v1", &legacy);
    assert_eq!(
        load_raw(&root, &legacy, full_adoption(&legacy, CANDIDATE_ID)),
        "catalog-schema-version-unsupported"
    );
}

#[cfg(unix)]
#[test]
pub(crate) fn symbolic_tool_executable_content_mode_and_identity_substitutions_are_refused() {
    let root = TestRoot::new("runner-substitution", VALID_CATALOG);
    let catalog = load_full(&root, CANDIDATE_ID);

    let metadata = fs::metadata("/usr/bin/true").unwrap();
    let forged = RunnerObservation::new(
        "ultragoal",
        TRUE_TOOL_ID,
        "/usr/bin/true",
        sha(b"different executable"),
        metadata.len(),
        metadata.mode(),
    )
    .unwrap();
    let request = CatalogSelectionRequest::new(
        catalog.catalog_id(),
        GRAPH_ID,
        CANDIDATE_ID,
        PLAN_ID,
        selected(&root, false),
        vec![forged],
    )
    .unwrap();
    assert_eq!(
        catalog.bind_selected(request).unwrap_err().code(),
        "catalog-runner-program-identity-stale"
    );

    let forged_mode = RunnerObservation::new(
        "ultragoal",
        TRUE_TOOL_ID,
        "/usr/bin/true",
        file_sha(Path::new("/usr/bin/true")),
        metadata.len(),
        metadata.mode() ^ 0o100,
    )
    .unwrap();
    let request = CatalogSelectionRequest::new(
        catalog.catalog_id(),
        GRAPH_ID,
        CANDIDATE_ID,
        PLAN_ID,
        selected(&root, false),
        vec![forged_mode],
    )
    .unwrap();
    assert_eq!(
        catalog.bind_selected(request).unwrap_err().code(),
        "catalog-runner-program-identity-stale"
    );
}
