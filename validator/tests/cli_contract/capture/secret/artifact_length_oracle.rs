#![cfg(target_os = "macos")]

use super::capture::{
    CommandSpec, PublicArtifact, SecretArg, capture_spec_artifacts_for_test,
    reset_test_descriptor_bytes_read, reset_test_file_open_attempts, test_descriptor_bytes_read,
    test_file_open_attempts,
};
use super::fixture::RepoFixture;
use crate::context::{BuildRequest, LiveContext};

fn context(fixture: &RepoFixture) -> LiveContext {
    LiveContext::build(
        BuildRequest::new(fixture.root())
            .expect_repository_root(fixture.root())
            .expect_worktree_root(fixture.root())
            .bind_secret_source("SECRET", "v1")
            .probe_tool("sandbox-exec")
            .probe_tool("true"),
    )
    .unwrap()
}

#[test]
fn secret_artifact_path_remains_zero_open_under_length_oracle_controls() {
    let fixture = RepoFixture::new("secret-length-artifact-zero-open");
    let context = context(&fixture);
    let spec = CommandSpec::catalog_read("secret-length-artifact", "true")
        .secret_arg(SecretArg::new("0064", "SECRET"))
        .public_artifact(PublicArtifact::new("out/64.bin"));
    reset_test_file_open_attempts();
    reset_test_descriptor_bytes_read();
    let artifacts = capture_spec_artifacts_for_test(&context, &spec).unwrap();
    assert_eq!(test_file_open_attempts(), 0);
    assert_eq!(test_descriptor_bytes_read(), 0);
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].relative_path(), None);
    assert!(artifacts[0].bytes().is_empty());
    let json = artifacts[0].to_canonical_json().unwrap();
    assert!(!String::from_utf8_lossy(&json).contains("64.bin"));
}
