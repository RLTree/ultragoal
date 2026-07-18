use std::io::{Error, ErrorKind};
use std::path::Path;

#[test]
fn package_artifact_boundary_result_contracts_are_testable() {
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
fn coverage_digest_boundary_contracts_are_testable() {
    assert!(
        crate::audit::coverage::scope::digests::source_rel_path(
            Path::new("/repo"),
            Path::new("/outside/file.rs")
        )
        .expect_err("outside source path rejected")
        .contains("source tree strip failed")
    );
    assert!(
        crate::audit::coverage::scope::digests::digest_file_bytes(
            "src/lib.rs",
            Err("forced read".to_string()),
        )
        .expect_err("coverage digest read fails")
        .contains("coverage digest read failed")
    );
}
