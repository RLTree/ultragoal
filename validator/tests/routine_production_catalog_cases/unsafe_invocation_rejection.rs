use super::*;

#[test]
pub(crate) fn unsafe_workdir_loader_environment_path_overlap_and_prose_substitution_fail_closed() {
    let bytes = one_node_catalog(
        &["--version".to_owned()],
        &[],
        &["src/input.txt".to_owned()],
        &["target/routine-syntax".to_owned()],
        "..",
    );
    let root = TestRoot::new("unsafe-workdir", &bytes);
    assert_eq!(
        load_raw(&root, &bytes, one_node_adoption(&bytes)),
        "catalog-working-directory-unsafe"
    );

    let bytes = one_node_catalog(
        &["--version".to_owned()],
        &[("DYLD_INSERT_LIBRARIES".to_owned(), "/tmp/x".to_owned())],
        &["src/input.txt".to_owned()],
        &["target/routine-syntax".to_owned()],
        ".",
    );
    let root = TestRoot::new("unsafe-environment", &bytes);
    assert_eq!(
        load_raw(&root, &bytes, one_node_adoption(&bytes)),
        "catalog-environment-invalid"
    );

    let bytes = one_node_catalog(
        &["--version".to_owned()],
        &[],
        &["target/input.txt".to_owned()],
        &["target".to_owned()],
        ".",
    );
    let root = TestRoot::new("overlapping-output", &bytes);
    assert_eq!(
        load_raw(&root, &bytes, one_node_adoption(&bytes)),
        "catalog-output-scope-overlaps-input"
    );

    let bytes = one_node_catalog(
        &["--version".to_owned()],
        &[],
        &["../secret.txt".to_owned()],
        &["target/routine-syntax".to_owned()],
        ".",
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

#[cfg(unix)]
#[test]
pub(crate) fn symbolic_tool_executable_content_mode_and_identity_substitutions_are_refused() {
    let root = TestRoot::new("runner-substitution", VALID_CATALOG);
    let mutable_program = root.path().join("mutable-runner");
    fs::write(&mutable_program, fs::read("/usr/bin/true").unwrap()).unwrap();
    fs::set_permissions(&mutable_program, fs::Permissions::from_mode(0o700)).unwrap();
    let catalog = load_full(&root, CANDIDATE_ID);

    let caller_consistent_false = runner("true", TRUE_TOOL_ID, Path::new("/usr/bin/false"));
    let request = CatalogSelectionRequest::new(
        catalog.catalog_id(),
        GRAPH_ID,
        CANDIDATE_ID,
        PLAN_ID,
        selected(&root, false),
        vec![caller_consistent_false],
    )
    .unwrap();
    assert_eq!(
        catalog.bind_selected(request).unwrap_err().code(),
        "catalog-runner-authority-stale"
    );

    let metadata = fs::metadata("/usr/bin/true").unwrap();
    let forged = RunnerObservation::new(
        "true",
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
        "catalog-runner-authority-stale"
    );

    let forged_mode = RunnerObservation::new(
        "true",
        TRUE_TOOL_ID,
        "/usr/bin/true",
        TRUE_PROGRAM_SHA256,
        SYSTEM_PROGRAM_BYTE_LENGTH,
        SYSTEM_PROGRAM_UNIX_MODE ^ 0o100,
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
        "catalog-runner-authority-stale"
    );

    let alternate_selected = vec![
        SelectedRoutineNode::new(
            "syntax",
            Vec::<String>::new(),
            "true",
            OTHER_CANDIDATE_ID,
            false,
            sha(b"syntax input identity"),
            vec![input(&root, "src/input.txt")],
        )
        .unwrap(),
        SelectedRoutineNode::new(
            "verify",
            vec!["syntax".to_owned()],
            "true",
            OTHER_CANDIDATE_ID,
            false,
            sha(b"verify input identity"),
            vec![
                input(&root, "src/input.txt"),
                input(&root, "tests/input.txt"),
            ],
        )
        .unwrap(),
    ];
    let alternate_runner = runner("true", OTHER_CANDIDATE_ID, Path::new("/usr/bin/true"));
    let request = CatalogSelectionRequest::new(
        catalog.catalog_id(),
        GRAPH_ID,
        CANDIDATE_ID,
        PLAN_ID,
        alternate_selected,
        vec![alternate_runner],
    )
    .unwrap();
    assert_eq!(
        catalog.bind_selected(request).unwrap_err().code(),
        "catalog-selection-runner-authority-stale"
    );

    let mutable = runner("true", TRUE_TOOL_ID, &mutable_program);
    let request = CatalogSelectionRequest::new(
        catalog.catalog_id(),
        GRAPH_ID,
        CANDIDATE_ID,
        PLAN_ID,
        selected(&root, false),
        vec![mutable],
    )
    .unwrap();
    assert_eq!(
        catalog.bind_selected(request).unwrap_err().code(),
        "catalog-runner-authority-stale"
    );
}
