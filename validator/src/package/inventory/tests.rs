use serde_json::json;
use std::path::Path;

fn write_manifest(root: &std::path::Path, resources: serde_json::Value) {
    std::fs::write(
        root.join("plugin-manifest-draft.json"),
        serde_json::to_vec(&json!({"resources": resources})).expect("manifest"),
    )
    .expect("write manifest");
}

#[test]
fn canonical_escape_guard_reports_outside_package() {
    let root = Path::new("/package/root");
    assert!(super::canonical_escape_error(root, Path::new("/package/root/a"), "a").is_none());
    assert_eq!(
        super::canonical_escape_error(root, Path::new("/outside/a"), "a"),
        Some("package path escapes package root: a".to_string())
    );
}

#[test]
fn package_digest_rejects_missing_directories_and_invalid_paths() {
    let root = crate::self_tests::boundaries::support::temp_root("package-inventory-digest");
    std::fs::create_dir_all(root.join("docs")).expect("docs");
    std::fs::write(root.join("docs/file.txt"), "ok").expect("file");

    write_manifest(&root, json!(["docs/file.txt"]));
    assert!(
        super::package_digest(&root)
            .expect("valid digest")
            .starts_with("sha256:")
    );

    write_manifest(&root, json!(["docs"]));
    assert!(
        super::package_digest(&root)
            .expect_err("directory rejected")
            .contains("package digest manifest path is missing")
    );

    write_manifest(&root, json!(["docs/missing.txt"]));
    assert!(
        super::package_digest(&root)
            .expect_err("missing file rejected")
            .contains("package digest manifest path is missing")
    );

    write_manifest(&root, json!(["../escape.txt"]));
    assert!(
        super::package_digest(&root)
            .expect_err("escape rejected")
            .contains("package digest path invalid")
    );
    std::fs::remove_dir_all(root).expect("cleanup package inventory");
}

#[test]
fn package_digest_excludes_mutable_final_packet_proof_receipt() {
    assert!(super::package_digest_excluded(
        "validation_artifacts/review/final-packet-proof.json"
    ));
    assert!(super::package_digest_excluded(
        "validation_artifacts/review/2026-06-25-session-log-hardening-packet.json"
    ));
    assert!(super::package_digest_excluded(
        "validation_artifacts/semantic-classification/minimal-goal-run/CLAIM-001.semantic-classification-receipt.json"
    ));
}

#[test]
fn package_digest_rejects_parent_session_contract_resources() {
    let root = crate::self_tests::boundaries::support::temp_root("package-parent-contract");
    std::fs::create_dir_all(root.join("docs")).expect("docs");
    std::fs::write(root.join("docs/package.md"), "package resource").expect("package doc");
    std::fs::write(
        root.join("docs/parent-session-full-ultragoal-compliance-prompt-2026-06-25.md"),
        "builder contract",
    )
    .expect("parent prompt");
    std::fs::create_dir_all(root.join("docs/ultragoal-contract-2026-07"))
        .expect("modular contract dir");
    std::fs::write(
        root.join("docs/ultragoal-contract-2026-07/README.md"),
        "modular builder contract",
    )
    .expect("modular contract");
    write_manifest(&root, json!(["docs/package.md"]));
    let before = super::package_digest(&root).expect("digest before");
    std::fs::write(
        root.join("docs/parent-session-full-ultragoal-compliance-prompt-2026-06-25.md"),
        "updated builder contract",
    )
    .expect("parent prompt update");
    std::fs::write(
        root.join("docs/ultragoal-contract-2026-07/README.md"),
        "updated modular builder contract",
    )
    .expect("modular contract update");
    let after = super::package_digest(&root).expect("digest after");
    assert_eq!(before, after);

    write_manifest(
        &root,
        json!(["docs/parent-session-full-ultragoal-compliance-prompt-2026-06-25.md"]),
    );
    let err = super::package_digest(&root).expect_err("parent contract listed");
    assert!(
        err.contains("parent-session contract is not a package resource"),
        "{err}"
    );
    write_manifest(&root, json!(["docs/ultragoal-contract-2026-07/README.md"]));
    let err = super::package_digest(&root).expect_err("modular parent contract listed");
    assert!(
        err.contains("parent-session contract is not a package resource"),
        "{err}"
    );
    std::fs::remove_dir_all(root).expect("cleanup parent contract digest");
}
