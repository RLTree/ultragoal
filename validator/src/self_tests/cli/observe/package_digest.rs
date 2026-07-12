use std::fs;

#[test]
fn package_digest_command_returns_read_only_observation_contract() {
    let root = super::minimal_root("package-digest-observability");
    let code = crate::command_run::run_with_exit_code(crate::Args {
        root: root.clone(),
        command: crate::Command::PackageDigest,
    })
    .expect("package digest command");
    assert_eq!(code, 0);
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    assert!(candidate.starts_with("sha256:"));
    assert!(!root.join("validation_artifacts").exists());
    fs::remove_dir_all(root).expect("cleanup package digest observability");
}

#[test]
fn package_digest_failure_is_read_only() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("package-digest-fail");
    fs::create_dir_all(&root).expect("root");
    let code = crate::command_run::run_with_exit_code(crate::Args {
        root: root.clone(),
        command: crate::Command::PackageDigest,
    })
    .expect("package digest command returns fail code");
    assert_eq!(code, 1);
    assert!(!root.join("validation_artifacts").exists());
    fs::remove_dir_all(root).expect("cleanup package digest fail");
}

#[test]
fn package_digest_ignores_unwritable_observability_output_paths() {
    let root = super::minimal_root("package-digest-write-fail");
    fs::write(root.join("validation_artifacts"), "not a directory").expect("block artifacts dir");
    assert_eq!(
        crate::cli::package::digest::run(&root).expect("read only"),
        0
    );
    fs::remove_dir_all(root).expect("cleanup package digest write fail");
}

#[cfg(unix)]
#[test]
fn package_digest_attack_returns_failure_without_receipt_or_supported_claim() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "package-digest-root-escape-attack",
    );
    fs::create_dir_all(root.join("docs")).expect("docs");
    fs::write(root.join("docs/file.txt"), b"trusted").expect("trusted");
    fs::write(
        root.join("plugin-manifest-draft.json"),
        br#"{"resources":["docs/file.txt"]}"#,
    )
    .expect("manifest");
    let replacement = root.join("replacement-docs");
    fs::create_dir(&replacement).expect("replacement");
    fs::write(replacement.join("file.txt"), b"SECRET_CANARY").expect("canary");
    let docs = root.join("docs");
    let held = root.join("docs-held");
    crate::package::inventory::anchored::test_hooks::set_before_component(
        "docs/file.txt",
        0,
        move || {
            fs::rename(&docs, &held).expect("hold docs");
            fs::rename(&replacement, &docs).expect("substitute docs");
        },
    );
    let code = crate::command_run::run_with_exit_code(crate::Args {
        root: root.clone(),
        command: crate::Command::PackageDigest,
    })
    .expect("attack returns typed failure");
    assert_eq!(code, 1, "attack must not support source_package_digest");
    assert!(!root.join("validation_artifacts").exists());
    fs::remove_dir_all(root).expect("cleanup attack root");
}
