use super::support::*;
use crate::migration::product::{
    ApplyAuthorizationAuthority, DurableMigrationStore, MigrationInputSource,
    issue_apply_authorization,
};
use std::ffi::CString;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, symlink};

#[test]
fn source_and_registry_substitution_refuse_with_zero_state_write() {
    for relative in [SOURCE_PATH, REGISTRY_PATH] {
        let fixture = TestHost::new(TestDisposition::Retirement);
        let mut adapters = fixture.open().unwrap();
        let original = adapters.source.capture().unwrap();
        let original_binding = original.binding();
        let before = fixture.state_bytes();
        overwrite(
            &fixture.repository.join(relative),
            b"synthetic substituted bytes\n",
        );
        let error = if relative == SOURCE_PATH {
            adapters.source.capture().unwrap_err()
        } else {
            adapters
                .source
                .revalidate(&original_binding, &[])
                .unwrap_err()
        };
        assert!(error.code().starts_with("migration-host-"));
        assert_eq!(fixture.state_bytes(), before);
        assert_eq!(adapters.store.effect_counts_for_test(), (0, 0));
    }
}

#[test]
fn symlink_hardlink_and_special_source_files_fail_closed() {
    {
        let fixture = TestHost::new(TestDisposition::Retirement);
        let mut adapters = fixture.open().unwrap();
        let source = fixture.repository.join(SOURCE_PATH);
        fs::remove_file(&source).unwrap();
        symlink(fixture.repository.join(TARGET_PATH), &source).unwrap();
        assert!(adapters.source.capture().is_err());
        assert_eq!(adapters.store.effect_counts_for_test(), (0, 0));
    }
    {
        let fixture = TestHost::new(TestDisposition::Retirement);
        let mut adapters = fixture.open().unwrap();
        fs::hard_link(
            fixture.repository.join(SOURCE_PATH),
            fixture.root.join("second-link"),
        )
        .unwrap();
        assert!(adapters.source.capture().is_err());
        assert_eq!(adapters.store.effect_counts_for_test(), (0, 0));
    }
    {
        let fixture = TestHost::new(TestDisposition::Retirement);
        let mut adapters = fixture.open().unwrap();
        let source = fixture.repository.join(SOURCE_PATH);
        fs::remove_file(&source).unwrap();
        let encoded = CString::new(source.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(encoded.as_ptr(), 0o600) }, 0);
        assert!(adapters.source.capture().is_err());
        assert_eq!(adapters.store.effect_counts_for_test(), (0, 0));
    }
}

#[test]
fn registry_symlink_and_hardlink_are_refused() {
    {
        let fixture = TestHost::new(TestDisposition::Pending);
        let mut adapters = fixture.open().unwrap();
        let registry = fixture.repository.join(REGISTRY_PATH);
        fs::remove_file(&registry).unwrap();
        symlink(fixture.repository.join(TARGET_PATH), registry).unwrap();
        assert!(adapters.source.capture().is_err());
    }
    {
        let fixture = TestHost::new(TestDisposition::Pending);
        let mut adapters = fixture.open().unwrap();
        fs::hard_link(
            fixture.repository.join(REGISTRY_PATH),
            fixture.root.join("registry-second-link"),
        )
        .unwrap();
        assert!(adapters.source.capture().is_err());
    }
}

#[test]
fn repository_and_state_root_rename_are_detected_from_anchored_descriptors() {
    {
        let fixture = TestHost::new(TestDisposition::Retirement);
        let mut adapters = fixture.open().unwrap();
        let before = fixture.state_bytes();
        fs::rename(&fixture.repository, fixture.root.join("repository-moved")).unwrap();
        assert!(adapters.source.capture().is_err());
        assert_eq!(fixture.state_bytes(), before);
    }
    {
        let fixture = TestHost::new(TestDisposition::Retirement);
        let adapters = fixture.open().unwrap();
        fs::rename(&fixture.state, fixture.root.join("state-moved")).unwrap();
        assert!(
            adapters
                .store
                .load_operation(&hash(b"unknown operation"))
                .is_err()
        );
    }
}

#[test]
fn source_and_state_permission_drift_fail_closed() {
    {
        let fixture = TestHost::new(TestDisposition::Retirement);
        let mut adapters = fixture.open().unwrap();
        let before = fixture.state_bytes();
        set_mode(&fixture.repository.join(SOURCE_PATH), 0o666);
        assert!(adapters.source.capture().is_err());
        assert_eq!(fixture.state_bytes(), before);
    }
    {
        let fixture = TestHost::new(TestDisposition::Retirement);
        let adapters = fixture.open().unwrap();
        set_mode(&fixture.state, 0o777);
        assert!(
            adapters
                .store
                .load_operation(&hash(b"unknown operation"))
                .is_err()
        );
    }
    {
        let fixture = TestHost::new(TestDisposition::Retirement);
        set_mode(&fixture.state.join("state.json"), 0o644);
        let error = match fixture.open() {
            Ok(_) => panic!("non-owner-only durable state unexpectedly opened"),
            Err(error) => error,
        };
        assert_eq!(error.code(), "migration-host-state-file-refused");
    }
}

#[test]
fn candidate_and_product_version_substitution_are_refused_at_open() {
    use crate::migration::product::DarwinMigrationHost;
    let fixture = TestHost::new(TestDisposition::Retirement);
    let wrong_candidate = hash(b"wrong candidate");
    let candidate_error = match DarwinMigrationHost::open(
        &fixture.repository,
        &fixture.state,
        fixture.inventory.clone(),
        &wrong_candidate,
        PRODUCT_VERSION,
    ) {
        Ok(_) => panic!("candidate substitution unexpectedly opened"),
        Err(error) => error,
    };
    assert_eq!(
        candidate_error.code(),
        "migration-host-inventory-candidate-refused"
    );
    let version_error = match DarwinMigrationHost::open(
        &fixture.repository,
        &fixture.state,
        fixture.inventory.clone(),
        &fixture.candidate_id,
        "0.0.13",
    ) {
        Ok(_) => panic!("product-version substitution unexpectedly opened"),
        Err(error) => error,
    };
    assert_eq!(
        version_error.code(),
        "migration-host-authority-binding-refused"
    );
}

#[test]
fn authority_key_and_ledger_substitution_are_detected() {
    for name in ["authority.json", "secret.key", "state.json"] {
        let fixture = TestHost::new(TestDisposition::Retirement);
        let adapters = fixture.open().unwrap();
        overwrite(&fixture.state.join(name), b"substituted\n");
        if name == "state.json" {
            assert!(
                adapters
                    .store
                    .load_operation(&hash(b"unknown operation"))
                    .is_err()
            );
        } else {
            assert!(adapters.boundary_authority().is_err());
        }
    }
}

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
    let source = include_str!("../../src/migration/product/host/effects.rs");
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
    let bytes = include_bytes!("../../../fixtures/migration-host-adapter/cases.json");
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
