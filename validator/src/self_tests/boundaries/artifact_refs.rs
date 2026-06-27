use std::io::{Error, ErrorKind};
use std::path::Path;

#[test]
fn artifact_boundary_result_helpers_are_testable() {
    let missing = Error::new(ErrorKind::NotFound, "forced missing");
    assert!(
        crate::target_repo::artifact_refs::target_root_result(Err(missing))
            .expect_err("root canonicalization fails")
            .contains("target repo root unavailable")
    );
    assert!(
        crate::target_repo::artifact_refs::artifact_canonical_result(
            "artifact",
            "proof.json",
            Err(Error::new(ErrorKind::NotFound, "forced missing")),
        )
        .expect_err("target artifact canonicalization fails")
        .contains("artifact missing: proof.json")
    );
    assert!(
        crate::target_repo::artifact_refs::artifact_metadata_result(
            "artifact",
            "proof.json",
            Err(Error::new(ErrorKind::NotFound, "forced metadata")),
        )
        .expect_err("target artifact metadata fails")
        .contains("artifact missing: proof.json")
    );
    assert!(
        crate::target_repo::artifact_refs::artifact_bytes_result(
            "artifact",
            Err("forced read".to_string()),
        )
        .expect_err("target artifact read fails")
        .contains("artifact unreadable")
    );
    assert!(
        crate::target_repo::artifact_refs::artifact_json_result(
            "artifact",
            serde_json::from_slice(b"{"),
        )
        .expect_err("target artifact json fails")
        .contains("json malformed")
    );
    assert!(
        crate::target_repo::artifact_refs::artifact_digest_result(Err("forced digest".to_string()))
            .expect_err("target artifact digest fails")
            .contains("forced digest")
    );
}

#[test]
fn package_artifact_boundary_result_helpers_are_testable() {
    assert!(
        crate::package::artifact::refs::package_root_result(Err(Error::new(
            ErrorKind::NotFound,
            "forced root",
        )))
        .expect_err("package root fails")
        .contains("package root unavailable")
    );
    assert!(
        crate::package::artifact::refs::package_artifact_canonical_result(
            "proof",
            "a.json",
            Err(Error::new(ErrorKind::NotFound, "forced canonical")),
        )
        .expect_err("package artifact canonicalization fails")
        .contains("artifact missing: a.json")
    );
    assert!(
        crate::package::artifact::refs::package_artifact_metadata_result(
            "proof",
            "a.json",
            Err(Error::new(ErrorKind::NotFound, "forced metadata")),
        )
        .expect_err("package artifact metadata fails")
        .contains("artifact missing: a.json")
    );
    assert!(
        crate::package::artifact::refs::package_artifact_digest_result(Err(
            "forced digest".to_string()
        ))
        .expect_err("package artifact digest fails")
        .contains("forced digest")
    );
}

#[test]
fn archive_zip_sync_result_is_testable() {
    assert!(crate::archive::zip::sync_result(Path::new("/tmp/archive.zip.tmp"), Ok(())).is_ok());
    assert!(
        crate::archive::zip::sync_result(
            Path::new("/tmp/archive.zip.tmp"),
            Err(Error::other("forced sync")),
        )
        .expect_err("zip sync fails")
        .contains("zip sync failed")
    );
}

#[test]
fn audit_artifact_digest_zero_is_testable() {
    let root = crate::self_tests::boundaries::support::temp_root("audit-artifact-digest-zero");
    std::fs::create_dir_all(root.join("dir")).expect("dir");
    std::fs::write(root.join("file.txt"), "content").expect("file");
    assert_eq!(
        crate::audit::artifacts::artifact_digest_or_zero(&root.join("dir"))
            .expect("directory digest fallback"),
        crate::digest::ZERO
    );
    assert_ne!(
        crate::audit::artifacts::artifact_digest_or_zero(&root.join("file.txt"))
            .expect("file digest"),
        crate::digest::ZERO
    );
    std::fs::remove_dir_all(root).expect("cleanup digest zero test");
}

#[test]
fn coverage_digest_boundary_helpers_are_testable() {
    assert!(
        crate::claim_semantics::coverage::digests::source_rel_path(
            Path::new("/repo"),
            Path::new("/outside/file.rs")
        )
        .expect_err("outside source path rejected")
        .contains("source tree strip failed")
    );
    assert!(
        crate::claim_semantics::coverage::digests::digest_file_bytes(
            "src/lib.rs",
            Err("forced read".to_string()),
        )
        .expect_err("coverage digest read fails")
        .contains("coverage digest read failed")
    );
}
