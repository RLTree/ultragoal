use serde_json::json;
use std::path::{Path, PathBuf};

fn package_root(label: &str) -> PathBuf {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
    std::fs::create_dir_all(root.join("docs")).expect("docs");
    std::fs::write(root.join("docs/file.txt"), "archive payload").expect("payload");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["docs/file.txt"]}),
    )
    .expect("manifest");
    root
}

fn build(root: &Path, zip: &Path, zip_root: &str) -> Result<serde_json::Value, String> {
    crate::archive::build_archive(root, zip, zip_root, "candidate_review_anchor")
}

#[test]
fn archive_build_rejects_invalid_zip_root_before_claiming_archive_proof() {
    let root = package_root("archive-build-invalid-zip-root");

    let error = build(&root, &root.join("candidate.zip"), "bad/root")
        .expect_err("invalid zip root fails closed");

    assert!(error.contains("archive zip root must be one directory name"));
    assert!(
        crate::archive::names::entry_name("bad/root", "docs/file.txt")
            .expect_err("entry names share zip-root validation")
            .contains("archive zip root must be one directory name")
    );
    std::fs::remove_dir_all(root).expect("cleanup invalid zip root");
}

#[test]
fn archive_build_reports_zip_write_failures_after_inputs_are_closed() {
    let root = package_root("archive-build-zip-write-fail");
    let blocked_parent = root.join("blocked-parent");
    std::fs::write(&blocked_parent, "not a directory").expect("blocked parent");

    let error = build(
        &root,
        &blocked_parent.join("candidate.zip"),
        "harness-ultragoal",
    )
    .expect_err("zip write failure fails closed");

    assert!(
        error.contains("create") || error.contains("parent"),
        "{error}"
    );
    assert!(
        !error.contains(&root.to_string_lossy().to_string()),
        "{error}"
    );
    std::fs::remove_dir_all(root).expect("cleanup zip write fail");
}
