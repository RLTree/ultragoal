use super::*;

#[cfg(unix)]
#[test]
pub(crate) fn source_path_escape_symlink_hardlink_special_and_non_utf8_are_refused_without_blocking()
 {
    super::catalog_fixture::run_catalog_case("source-path-security", |invocation| {
        let mut root = invocation.new_root("source-path-security", VALID_CATALOG)?;
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
        std::os::unix::fs::symlink("original.json", root.path().join("config/routines.json"))
            .unwrap();
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
        let fifo =
            CString::new(root.path().join("config/routines.json").to_str().unwrap()).unwrap();
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
        root.teardown_after_assertions();
        Ok(())
    });
}

#[cfg(unix)]
#[test]
pub(crate) fn transitive_input_symlink_hardlink_and_special_file_are_refused() {
    super::catalog_fixture::run_catalog_case("input-security", |invocation| {
        let mut root = invocation.new_root("input-symlink", VALID_CATALOG)?;
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
            vec![runner(
                "ultragoal",
                TRUE_TOOL_ID,
                Path::new("/usr/bin/true"),
            )],
        )
        .unwrap();
        assert_eq!(
            catalog.bind_selected(request).unwrap_err().code(),
            "catalog-file-object-unsafe"
        );
        drop(catalog);
        root.teardown_after_assertions();

        let mut root = invocation.new_root("input-hardlink", VALID_CATALOG)?;
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
            vec![runner(
                "ultragoal",
                TRUE_TOOL_ID,
                Path::new("/usr/bin/true"),
            )],
        )
        .unwrap();
        assert_eq!(
            catalog.bind_selected(request).unwrap_err().code(),
            "catalog-file-object-unsafe"
        );
        drop(catalog);
        root.teardown_after_assertions();

        let mut root = invocation.new_root("input-special", VALID_CATALOG)?;
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
            vec![runner(
                "ultragoal",
                TRUE_TOOL_ID,
                Path::new("/usr/bin/true"),
            )],
        )
        .unwrap();
        assert_eq!(
            catalog.bind_selected(request).unwrap_err().code(),
            "catalog-file-object-unsafe"
        );
        drop(catalog);
        root.teardown_after_assertions();

        let mut root = invocation.new_root("output-symlink", VALID_CATALOG)?;
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
        drop(catalog);
        root.teardown_after_assertions();
        Ok(())
    });
}

#[test]
pub(crate) fn catalog_input_and_output_bounds_are_enforced() {
    super::catalog_fixture::run_catalog_case("catalog-bounds", |invocation| {
        let reads = (0..129)
            .map(|index| format!("src/input-{index}.txt"))
            .collect::<Vec<_>>();
        let bytes = one_node_catalog(&reads, &["target/routine-syntax".to_owned()]);
        let mut root = invocation.new_root("read-bound", &bytes)?;
        assert_eq!(
            load_raw(&root, &bytes, one_node_adoption(&bytes)),
            "catalog-read-source-limit-exceeded"
        );
        root.teardown_after_assertions();

        let outputs = (0..129)
            .map(|index| format!("target/routine-{index}"))
            .collect::<Vec<_>>();
        let bytes = one_node_catalog(&["src/input.txt".to_owned()], &outputs);
        let mut root = invocation.new_root("output-bound", &bytes)?;
        assert_eq!(
            load_raw(&root, &bytes, one_node_adoption(&bytes)),
            "catalog-output-scope-limit-exceeded"
        );
        root.teardown_after_assertions();

        let too_large =
            TransitiveInputExpectation::new("src/input.txt", sha(b"x"), 64 * 1024 * 1024 + 1)
                .unwrap_err();
        assert_eq!(too_large.code(), "catalog-input-length-invalid");

        let oversized_catalog = vec![b'#'; 1024 * 1024 + 1];
        let mut root = invocation.new_root("catalog-bound", &oversized_catalog)?;
        assert_eq!(
            CatalogAdoption::new(
                sha(&oversized_catalog),
                oversized_catalog.len() as u64,
                GRAPH_ID,
                CANDIDATE_ID,
                vec![AdoptedRoutineNode::new("syntax", Vec::<String>::new()).unwrap()],
            )
            .unwrap_err()
            .code(),
            "catalog-source-length-invalid"
        );
        root.teardown_after_assertions();
        Ok(())
    });
}
