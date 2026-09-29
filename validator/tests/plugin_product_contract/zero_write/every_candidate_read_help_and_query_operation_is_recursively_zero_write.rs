#[cfg(unix)]
#[test]
fn every_candidate_read_help_and_query_operation_is_recursively_zero_write() {
    let repository = Repository::new();
    let operations = operations();
    assert_eq!(operations.len(), 16);
    let initial = observe(&repository.root);
    assert!(!initial.status.is_empty(), "fixture must remain dirty");
    for operation in operations {
        assert!(!operation.source_marker.is_empty());
        let before = observe(&repository.root);
        let output = execute(&repository.root, &operation.args);
        let after = observe(&repository.root);
        assert_eq!(after, before, "hidden write from {}", operation.id);
        let code = output
            .status
            .code()
            .unwrap_or_else(|| panic!("{} terminated without a process exit code", operation.id));
        assert!(
            // A read/query route may reject malformed or incomplete input at
            // parse/admission time. That invalid-invocation outcome must be
            // held to the same recursive zero-write guarantee as a successful
            // read; the operation catalog includes `eval audit`, whose typed
            // specification is intentionally required.
            matches!(code, 0 | 1 | 2 | 3 | 4),
            "{}: {output:?}",
            operation.id
        );
        if operation.id == "help-json" {
            assert_eq!(code, 0);
        }
        let mut emitted = output.stdout;
        emitted.extend_from_slice(&output.stderr);
        let text = String::from_utf8_lossy(&emitted);
        assert!(!text.contains(SECRET), "secret echo from {}", operation.id);
        for private in PRIVATE_ARGUMENTS {
            if operation.args.iter().any(|arg| arg == private) {
                assert!(
                    !text.contains(private),
                    "operator argument echo from {}",
                    operation.id
                );
            }
        }
    }
    assert_eq!(observe(&repository.root), initial);
}

#[cfg(unix)]
#[test]
fn unavailable_root_does_not_echo_the_operator_path_or_mutate_its_parent() {
    let repository = Repository::new();
    let missing = repository.root.join("private-missing-root-closure-018");
    let args = vec![
        "--root".to_owned(),
        missing.to_string_lossy().into_owned(),
        "--json".to_owned(),
        "inspect".to_owned(),
        "context".to_owned(),
    ];
    let before = observe(&repository.root);
    let output = execute(&repository.root, &args);
    let after = observe(&repository.root);
    assert_eq!(after, before);
    assert_eq!(output.status.code(), Some(4));
    let mut emitted = output.stdout;
    emitted.extend_from_slice(&output.stderr);
    let text = String::from_utf8_lossy(&emitted);
    assert!(!text.contains("private-missing-root-closure-018"));
    assert!(!text.contains(SECRET));
}

#[cfg(unix)]
#[test]
fn lifecycle_plan_verify_and_build_closure_capture_are_recursively_zero_write() {
    use super::plugin_product::lifecycle::{
        LifecycleAuthorization, LifecycleIntent, LifecycleRequest, LifecycleState,
        PackageAuthority, Version, plan,
    };
    use super::plugin_product::source_closure::{
        BuildClosurePolicy, BuildClosureV1, BuildInputKind, RequiredBuildInput,
    };

    let repository = Repository::new();
    let policy = BuildClosurePolicy::new(vec![
        RequiredBuildInput {
            path: ".codex-plugin/plugin.json".to_owned(),
            kind: BuildInputKind::RuntimeAuthority,
        },
        RequiredBuildInput {
            path: "nested/tracked.txt".to_owned(),
            kind: BuildInputKind::RustSource,
        },
        RequiredBuildInput {
            path: "private-canary.txt".to_owned(),
            kind: BuildInputKind::VerifierInput,
        },
    ])
    .unwrap();
    let before = observe(&repository.root);
    let closure = BuildClosureV1::capture(&repository.root, &policy).unwrap();
    closure.verify(&repository.root, &policy).unwrap();

    let authority = PackageAuthority {
        version: Version::parse("0.0.12").unwrap(),
        package_sha256: "sha256:1111111111111111111111111111111111111111111111111111111111111111"
            .to_owned(),
        inventory_sha256: "sha256:2222222222222222222222222222222222222222222222222222222222222222"
            .to_owned(),
        candidate_id: "sha256:3333333333333333333333333333333333333333333333333333333333333333"
            .to_owned(),
    };
    let request = LifecycleRequest {
        intent: LifecycleIntent::RepeatUse,
        target: Some(authority.clone()),
        prior_authority: None,
        authorization: LifecycleAuthorization {
            allow_host_write: false,
            allow_downgrade: false,
            expected_installed_sha256: Some(authority.package_sha256.clone()),
        },
    };
    let state = LifecycleState {
        installed: Some(authority.clone()),
        cache: Some(authority),
        generation: 1,
        recovery_required: false,
    };
    let planned = plan(&state, &request).unwrap();
    assert!(!planned.writes_host_state);
    assert_eq!(planned.expected_after, state);

    assert_eq!(observe(&repository.root), before);
}
