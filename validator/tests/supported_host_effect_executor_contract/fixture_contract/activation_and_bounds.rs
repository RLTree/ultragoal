fn assert_activation_and_bounds(cases: &Cases) {
    assert_eq!(cases.schema_version, "SupportedHostEffectExecutorCases-v1");
    assert_eq!(cases.activation.module_visibility, "crate-private");
    assert_eq!(cases.activation.public_constructor_count, 0);
    assert!(cases.activation.root_module_wiring_required);
    assert!(!cases.activation.public_cli_route_added);
    assert_eq!(
        cases.activation.darwin_external_process_execution,
        "unsupported-before-fork-spawn-write"
    );
    assert!(
        cases
            .activation
            .darwin_descriptor_relative_publication_tested_with_in_process_backend
    );
    assert_eq!(
        cases.activation.lifecycle_recovery_reexports_required,
        [
            "ExpectedPublicationObjectIdentity",
            "PublicationAcknowledgementIdentity",
            "PublicationExpectation",
        ]
    );
    assert_eq!(cases.supported_process_platforms.len(), 2);
    assert_eq!(
        cases
            .supported_process_platforms
            .iter()
            .map(|row| (row.platform.as_str(), row.primitive.as_str()))
            .collect::<Vec<_>>(),
        [("linux", "execveat-empty-path"), ("freebsd", "fexecve")]
    );
    assert!(
        cases
            .supported_process_platforms
            .iter()
            .all(|row| !row.live_platform_tested)
    );
    assert_eq!(cases.transaction_order.len(), 12);

    let bounds = &cases.execution_bounds;
    assert!(!bounds.environment_inherited);
    assert_eq!(bounds.accepted_environment_entries, 0);
    assert_eq!(bounds.stdout_limit_bytes, 1_048_576);
    assert_eq!(bounds.stderr_limit_bytes, 1_048_576);
    assert_eq!(bounds.maximum_timeout_ms, 300_000);
    assert!(!bounds.shell_execution);
    assert!(bounds.process_group_containment);
    assert_eq!(bounds.target_root_parent, "/private/tmp");
    assert_eq!(bounds.target_root_mode, "0700");
    assert_eq!(bounds.publication_mode, "0400");
    assert_eq!(bounds.publication_hard_links, 1);
    assert_eq!(bounds.publication_maximum_bytes, 1_048_576);
}
