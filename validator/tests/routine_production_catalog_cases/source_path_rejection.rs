use super::*;

#[cfg(unix)]
#[test]
pub(crate) fn source_path_escape_symlink_hardlink_special_and_non_utf8_are_refused_without_blocking()
 {
    let root = TestRoot::new("source-path-security", VALID_CATALOG);
    assert_eq!(
        load_production_catalog(
            root.path(),
            Path::new("../outside.json"),
            full_adoption(VALID_CATALOG, CANDIDATE_ID),
        )
        .unwrap_err()
        .code(),
        "catalog-repository-relative-path-required"
    );

    let original = root.path().join("config/original.json");
    fs::rename(root.path().join("config/routines.json"), &original).unwrap();
    std::os::unix::fs::symlink("original.json", root.path().join("config/routines.json")).unwrap();
    assert_eq!(
        load_production_catalog(
            root.path(),
            Path::new("config/routines.json"),
            full_adoption(VALID_CATALOG, CANDIDATE_ID),
        )
        .unwrap_err()
        .code(),
        "catalog-file-object-unsafe"
    );

    fs::remove_file(root.path().join("config/routines.json")).unwrap();
    fs::hard_link(&original, root.path().join("config/routines.json")).unwrap();
    assert_eq!(
        load_production_catalog(
            root.path(),
            Path::new("config/routines.json"),
            full_adoption(VALID_CATALOG, CANDIDATE_ID),
        )
        .unwrap_err()
        .code(),
        "catalog-file-object-unsafe"
    );

    fs::remove_file(root.path().join("config/routines.json")).unwrap();
    let fifo = CString::new(root.path().join("config/routines.json").to_str().unwrap()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    assert_eq!(
        load_production_catalog(
            root.path(),
            Path::new("config/routines.json"),
            full_adoption(VALID_CATALOG, CANDIDATE_ID),
        )
        .unwrap_err()
        .code(),
        "catalog-file-object-unsafe"
    );

    let non_utf8 = PathBuf::from(std::ffi::OsString::from_vec(vec![b'c', 0xff]));
    assert_eq!(
        load_production_catalog(
            root.path(),
            &non_utf8,
            full_adoption(VALID_CATALOG, CANDIDATE_ID),
        )
        .unwrap_err()
        .code(),
        "catalog-source-path-not-utf8"
    );
}

#[cfg(unix)]
#[test]
pub(crate) fn transitive_input_symlink_hardlink_and_special_file_are_refused() {
    let root = TestRoot::new("input-symlink", VALID_CATALOG);
    let catalog = load_full(&root, CANDIDATE_ID);
    let stale = selected(&root, false);
    fs::rename(
        root.path().join("tests/input.txt"),
        root.path().join("tests/original.txt"),
    )
    .unwrap();
    std::os::unix::fs::symlink("original.txt", root.path().join("tests/input.txt")).unwrap();
    let request = CatalogSelectionRequest::new(
        catalog.catalog_id(),
        GRAPH_ID,
        CANDIDATE_ID,
        PLAN_ID,
        stale,
        vec![runner("true", TRUE_TOOL_ID, Path::new("/usr/bin/true"))],
    )
    .unwrap();
    assert_eq!(
        catalog.bind_selected(request).unwrap_err().code(),
        "catalog-file-object-unsafe"
    );

    let root = TestRoot::new("input-hardlink", VALID_CATALOG);
    let catalog = load_full(&root, CANDIDATE_ID);
    let stale = selected(&root, false);
    fs::hard_link(
        root.path().join("tests/input.txt"),
        root.path().join("tests/input-alias.txt"),
    )
    .unwrap();
    let request = CatalogSelectionRequest::new(
        catalog.catalog_id(),
        GRAPH_ID,
        CANDIDATE_ID,
        PLAN_ID,
        stale,
        vec![runner("true", TRUE_TOOL_ID, Path::new("/usr/bin/true"))],
    )
    .unwrap();
    assert_eq!(
        catalog.bind_selected(request).unwrap_err().code(),
        "catalog-file-object-unsafe"
    );

    let root = TestRoot::new("input-special", VALID_CATALOG);
    let catalog = load_full(&root, CANDIDATE_ID);
    let stale = selected(&root, false);
    fs::remove_file(root.path().join("tests/input.txt")).unwrap();
    let fifo = CString::new(root.path().join("tests/input.txt").to_str().unwrap()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    let request = CatalogSelectionRequest::new(
        catalog.catalog_id(),
        GRAPH_ID,
        CANDIDATE_ID,
        PLAN_ID,
        stale,
        vec![runner("true", TRUE_TOOL_ID, Path::new("/usr/bin/true"))],
    )
    .unwrap();
    assert_eq!(
        catalog.bind_selected(request).unwrap_err().code(),
        "catalog-file-object-unsafe"
    );

    let root = TestRoot::new("output-symlink", VALID_CATALOG);
    fs::create_dir_all(root.path().join("target/alias-destination")).unwrap();
    fs::remove_dir(root.path().join("target/routine-verify")).unwrap();
    std::os::unix::fs::symlink(
        "alias-destination",
        root.path().join("target/routine-verify"),
    )
    .unwrap();
    let catalog = load_full(&root, CANDIDATE_ID);
    let request = self::request(&catalog, &root, CANDIDATE_ID, false);
    assert_eq!(
        catalog.bind_selected(request).unwrap_err().code(),
        "catalog-output-scope-unsafe"
    );
}

#[test]
pub(crate) fn catalog_argv_environment_input_and_output_bounds_are_enforced() {
    let arguments = (0..129)
        .map(|index| format!("arg-{index}"))
        .collect::<Vec<_>>();
    let bytes = one_node_catalog(
        &arguments,
        &[],
        &["src/input.txt".to_owned()],
        &["target/routine-syntax".to_owned()],
        ".",
    );
    let root = TestRoot::new("argv-bound", &bytes);
    assert_eq!(
        load_raw(&root, &bytes, one_node_adoption(&bytes)),
        "catalog-arguments-invalid"
    );

    let environment = (0..62)
        .map(|index| (format!("ROUTINE_{index}"), "v".to_owned()))
        .collect::<Vec<_>>();
    let bytes = one_node_catalog(
        &["--version".to_owned()],
        &environment,
        &["src/input.txt".to_owned()],
        &["target/routine-syntax".to_owned()],
        ".",
    );
    let root = TestRoot::new("environment-bound", &bytes);
    assert_eq!(
        load_raw(&root, &bytes, one_node_adoption(&bytes)),
        "catalog-environment-invalid"
    );

    let reads = (0..129)
        .map(|index| format!("src/input-{index}.txt"))
        .collect::<Vec<_>>();
    let bytes = one_node_catalog(
        &["--version".to_owned()],
        &[],
        &reads,
        &["target/routine-syntax".to_owned()],
        ".",
    );
    let root = TestRoot::new("read-bound", &bytes);
    assert_eq!(
        load_raw(&root, &bytes, one_node_adoption(&bytes)),
        "catalog-read-source-limit-exceeded"
    );

    let outputs = (0..129)
        .map(|index| format!("target/routine-{index}"))
        .collect::<Vec<_>>();
    let bytes = one_node_catalog(
        &["--version".to_owned()],
        &[],
        &["src/input.txt".to_owned()],
        &outputs,
        ".",
    );
    let root = TestRoot::new("output-bound", &bytes);
    assert_eq!(
        load_raw(&root, &bytes, one_node_adoption(&bytes)),
        "catalog-output-scope-limit-exceeded"
    );

    let too_large =
        TransitiveInputExpectation::new("src/input.txt", sha(b"x"), 64 * 1024 * 1024 + 1)
            .unwrap_err();
    assert_eq!(too_large.code(), "catalog-input-length-invalid");

    let oversized_catalog = vec![b'#'; 1024 * 1024 + 1];
    let root = TestRoot::new("catalog-bound", &oversized_catalog);
    assert_eq!(
        CatalogAdoption::new(
            sha(&oversized_catalog),
            oversized_catalog.len() as u64,
            GRAPH_ID,
            CANDIDATE_ID,
            vec![AdoptedRoutineNode::new("syntax", Vec::<String>::new(), "true", None).unwrap()],
        )
        .unwrap_err()
        .code(),
        "catalog-source-length-invalid"
    );
    drop(root);
}
