use super::support::{metadata_snapshot, resource_root, root, write_manifest};
use crate::package::inventory::{anchored, package_digest};
use std::fs;

#[test]
fn package_session_enforces_path_entry_and_aggregate_limits() {
    let root = resource_root("package-session-limits", "file.txt", b"");
    let mut session = anchored::Session::open(&root).expect("session");
    let long = format!("{}.txt", "a".repeat(4096));
    assert!(
        session
            .read(&long, anchored::MAX_RESOURCE_BYTES)
            .expect_err("long path")
            .contains("path exceeds")
    );
    session.set_usage_for_test(100_000, 0);
    assert!(
        session
            .read("file.txt", anchored::MAX_RESOURCE_BYTES)
            .expect_err("entry bound")
            .contains("entry limit")
    );

    let mut session = anchored::Session::open(&root).expect("second session");
    session.set_usage_for_test(0, 512 * 1024 * 1024 + 1);
    assert!(
        session
            .read("file.txt", anchored::MAX_RESOURCE_BYTES)
            .expect_err("aggregate bound")
            .contains("session exceeds its byte limit")
    );
    fs::remove_dir_all(root).expect("cleanup limits");
}

#[test]
fn package_digest_rejects_oversized_resource_from_metadata() {
    let root = root("package-resource-size-limit");
    let file = fs::File::create(root.join("large.bin")).expect("large file");
    file.set_len(anchored::MAX_RESOURCE_BYTES + 1)
        .expect("sparse length");
    write_manifest(&root, &["large.bin"]);
    let error = package_digest(&root).expect_err("large resource rejected");
    assert!(error.contains("file exceeds its byte limit"), "{error}");
    fs::remove_dir_all(root).expect("cleanup large resource");
}

#[test]
fn package_manifest_rejects_duplicate_keys_before_resource_reads() {
    let root = root("package-manifest-duplicate-key");
    fs::write(root.join("a.txt"), b"a").expect("a");
    fs::write(root.join("b.txt"), b"b").expect("b");
    fs::write(
        root.join("plugin-manifest-draft.json"),
        br#"{"resources":["a.txt"],"resources":["b.txt"]}"#,
    )
    .expect("duplicate manifest");
    assert!(
        package_digest(&root)
            .expect_err("duplicate manifest rejected")
            .contains("duplicate keys")
    );
    fs::remove_dir_all(root).expect("cleanup duplicate manifest");
}

#[test]
fn package_digest_is_read_only_and_deterministic() {
    let root = resource_root("package-session-read-only", "docs/file.txt", b"trusted");
    let before = metadata_snapshot(&root);
    let first = package_digest(&root).expect("first digest");
    let second = package_digest(&root).expect("second digest");
    assert_eq!(first, second);
    assert_eq!(before, metadata_snapshot(&root));
    assert!(!root.join("validation_artifacts").exists());
    fs::remove_dir_all(root).expect("cleanup deterministic root");
}
