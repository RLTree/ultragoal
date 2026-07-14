#[test]
fn maximum_registry_and_source_bounds_are_enforced_without_state_write() {
    for (relative, size) in [
        (REGISTRY_PATH, 2 * 1024 * 1024 + 1),
        (SOURCE_PATH, 8 * 1024 * 1024 + 1),
    ] {
        let fixture = TestHost::new(TestDisposition::Retirement);
        let mut adapters = fixture.open().unwrap();
        let before = fixture.state_bytes();
        overwrite(&fixture.repository.join(relative), &vec![b'x'; size]);
        assert!(adapters.source.capture().is_err());
        assert_eq!(fixture.state_bytes(), before);
    }
}

#[test]
fn crossed_trusted_deadline_refuses_compatibility_with_zero_effect_and_zero_write() {
    let fixture = TestHost::new(TestDisposition::Compatibility);
    let mut adapters = fixture.open().unwrap();
    let plan = fixture.derive_plan(&mut adapters);
    let before = fixture.state_bytes();
    adapters.advance_trusted_time_for_test(2 * 86_400_000);
    let mut authority = adapters.apply_authority(&plan).unwrap();
    let error =
        issue_apply_authorization(&plan, &mut adapters.source, &mut authority, &adapters.store)
            .unwrap_err();
    assert_eq!(
        error.code(),
        "migration-product-compatibility-boundary-crossed"
    );
    assert_eq!(fixture.state_bytes(), before);
    assert_eq!(adapters.store.effect_counts_for_test(), (0, 0));
}

#[test]
fn authority_sessions_and_nonces_are_os_random_per_issue() {
    let fixture = TestHost::new(TestDisposition::Retirement);
    let mut adapters = fixture.open().unwrap();
    let plan = fixture.derive_plan(&mut adapters);
    let first = adapters.apply_authority(&plan).unwrap();
    let second = adapters.apply_authority(&plan).unwrap();
    assert_ne!(first.session_id(), second.session_id());
    assert_ne!(first.nonce_sha256(), second.nonce_sha256());
}

#[test]
fn durable_state_remains_owner_only_regular_and_single_link() {
    let fixture = TestHost::new(TestDisposition::Retirement);
    let adapters = fixture.open().unwrap();
    let metadata = fs::symlink_metadata(fixture.state.join("state.json")).unwrap();
    assert!(metadata.file_type().is_file());
    assert_eq!(metadata.mode() & 0o777, 0o600);
    assert_eq!(metadata.nlink(), 1);
    assert_eq!(adapters.store.effect_counts_for_test(), (0, 0));
}

#[test]
fn confined_effect_adapter_has_no_path_shell_or_delete_primitive() {
    let source = include_str!("../../../src/migration/product/host/effects.rs");
    for forbidden in [
        "std::fs",
        "std::path",
        "Command",
        "remove_file",
        "remove_dir",
        "unlink",
        "rename",
        "openat",
    ] {
        assert!(
            !source.contains(forbidden),
            "confined effect source contains forbidden primitive {forbidden}"
        );
    }
}

#[test]
fn synthetic_fixture_manifest_names_false_pass_and_security_controls() {
    let bytes = include_bytes!("../../../../fixtures/migration-host-adapter/cases.json");
    let value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    assert_eq!(value["schema_version"], "MigrationHostAdapterFixtures-v1");
    assert_eq!(value["real_user_state"], false);
    assert_eq!(value["destructive_cleanup_authorized"], false);
    for control in [
        "proof-artifact",
        "receipt-production",
        "score-only",
        "test-manipulation",
        "verbosity",
    ] {
        assert!(
            value["false_pass_controls"]
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| entry == control)
        );
    }
}
