#[test]
fn dep_info_and_policy_reject_outside_root_and_parent_paths() {
    use super::plugin_product::source_closure::{
        BuildClosurePolicy, BuildInputKind, ClosureError, RequiredBuildInput,
    };
    let root = ClosureTempRoot::new();
    let outside = std::env::temp_dir().join(format!("hul-outside-{}.rs", std::process::id()));
    std::fs::write(&outside, b"outside\n").unwrap();
    std::fs::write(
        root.path().join("target/outside.d"),
        format!("target/probe: {}\n", outside.display()),
    )
    .unwrap();
    assert_eq!(
        BuildClosurePolicy::from_dep_info(
            root.path(),
            &["target/outside.d".to_owned()],
            Vec::new(),
        ),
        Err(ClosureError::OutsideRoot)
    );
    let _ = std::fs::remove_file(outside);
    assert_eq!(
        BuildClosurePolicy::new(vec![RequiredBuildInput {
            path: "../escape".to_owned(),
            kind: BuildInputKind::DynamicInput,
        }]),
        Err(ClosureError::InvalidPath)
    );
}

#[test]
fn declared_plugin_product_inputs_are_current_and_byte_identical_twice() {
    use super::plugin_product::source_closure::{BuildClosureV1, plugin_product_build_policy};
    use std::collections::BTreeSet;

    let policy = plugin_product_build_policy().unwrap();
    let first = BuildClosureV1::capture(&root(), &policy).unwrap();
    let second = BuildClosureV1::capture(&root(), &policy).unwrap();
    if std::env::var_os("HUL_PRINT_BUILD_CLOSURE").is_some() {
        println!("{}", serde_json::to_string(&first).unwrap());
    }
    assert_eq!(first, second);
    assert_eq!(first.rows.len(), policy.required_inputs.len());
    let paths = first
        .rows
        .iter()
        .map(|row| row.path.as_str())
        .collect::<BTreeSet<_>>();
    assert!(paths.contains("validator/src/cli/successor/catalog/mod.rs"));
    assert!(paths.contains("validator/src/cli/successor/catalog/inspection.rs"));
    assert!(paths.contains("validator/src/cli/successor/catalog/repository_fit_and_checks.rs"));
    assert!(paths.contains("validator/src/cli/successor/catalog/observability_and_package.rs"));
    assert!(paths.contains("validator/src/cli/successor/catalog/evaluation_and_migration.rs"));
    assert!(paths.contains("validator/src/cli/successor/catalog/options.rs"));
    assert!(paths.contains("validator/src/lib.rs"));
    assert!(paths.contains("fixtures/plugin-product/root-wiring-request.json"));
    assert!(paths.iter().all(|path| {
        !path.contains("worker-results/")
            && !path.contains("work-packages/")
            && !path.contains("receipt/")
    }));
    first.verify(&root(), &policy).unwrap();
}
