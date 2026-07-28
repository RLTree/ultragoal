use super::*;

#[cfg(unix)]
#[test]
pub(crate) fn omitted_or_injected_transitive_inputs_and_definition_only_targets_cannot_pass() {
    super::catalog_fixture::run_catalog_case("input-set-inexact", |invocation| {
        let mut root = invocation.new_root("input-set-inexact", VALID_CATALOG)?;
        let catalog = load_full(&root, CANDIDATE_ID);
        let mut rows = selected(&root, false);
        rows[1] = SelectedRoutineNode::new(
            "verify",
            vec!["syntax".to_owned()],
            sha(b"verify input identity"),
            vec![input(&root, "src/input.txt")],
        )
        .unwrap();
        let request = CatalogSelectionRequest::new(
            catalog.catalog_id(),
            GRAPH_ID,
            CANDIDATE_ID,
            PLAN_ID,
            rows,
            vec![runner(
                "ultragoal",
                TRUE_TOOL_ID,
                Path::new("/usr/bin/true"),
            )],
        )
        .unwrap();
        assert_eq!(
            catalog.bind_selected(request).unwrap_err().code(),
            "catalog-selection-transitive-input-set-inexact"
        );

        assert_eq!(
            CatalogSelectionRequest::new(
                catalog.catalog_id(),
                GRAPH_ID,
                CANDIDATE_ID,
                PLAN_ID,
                Vec::new(),
                Vec::new(),
            )
            .unwrap_err()
            .code(),
            "catalog-selection-cardinality-invalid"
        );

        let extra_runner = CatalogSelectionRequest::new(
            catalog.catalog_id(),
            GRAPH_ID,
            CANDIDATE_ID,
            PLAN_ID,
            selected(&root, false),
            vec![
                runner("ultragoal", TRUE_TOOL_ID, Path::new("/usr/bin/true")),
                runner("other", OTHER_CANDIDATE_ID, Path::new("/usr/bin/false")),
            ],
        )
        .unwrap_err();
        assert_eq!(
            extra_runner.code(),
            "catalog-runner-observation-set-inexact"
        );
        drop(catalog);
        root.teardown_after_assertions();
        Ok(())
    });
}

#[cfg(unix)]
#[test]
pub(crate) fn parse_query_and_refusal_paths_are_recursively_zero_write() {
    super::catalog_fixture::run_catalog_case("zero-write", |invocation| {
        let mut root = invocation.new_root("zero-write", VALID_CATALOG)?;
        commit_fixture(root.path());
        let before = tree(root.path());
        let before_status = status(root.path());
        let catalog = load_full(&root, CANDIDATE_ID);
        assert_eq!(catalog.graph_id(), GRAPH_ID);
        assert_eq!(catalog.definition_count(), 2);
        assert_eq!(tree(root.path()), before);
        assert_eq!(status(root.path()), before_status);

        let metadata = fs::metadata("/usr/bin/true").unwrap();
        let forged = RunnerObservation::new(
            "ultragoal",
            TRUE_TOOL_ID,
            "/usr/bin/true",
            sha(b"forged"),
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
        assert!(catalog.bind_selected(request).is_err());
        assert_eq!(tree(root.path()), before);
        assert_eq!(status(root.path()), before_status);
        drop(catalog);
        root.teardown_after_assertions();
        Ok(())
    });
}
