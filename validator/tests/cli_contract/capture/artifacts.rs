use super::capture::{
    ArtifactResolver, CapturedArtifact, PublicArtifact, SecretArtifact, capture_public_for_test,
    reset_test_descriptor_bytes_read, set_test_artifact_pause_ms, test_descriptor_bytes_read,
};
use super::fixture::{RepoFixture, read_command};
use crate::context::LiveContext;
use sha2::{Digest, Sha256};
use std::fs;
use std::time::Duration;

#[cfg(unix)]
use std::os::unix::fs::symlink;

const MIB: usize = 1024 * 1024;

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn capture(
    context: &LiveContext,
    artifacts: Vec<PublicArtifact>,
) -> Result<Vec<CapturedArtifact>, String> {
    capture_public_for_test(context, artifacts)
}

#[test]
fn captured_artifact_is_an_immutable_self_authenticating_memory_record() {
    let fixture = RepoFixture::new("artifact-immutable");
    fixture.write_file("out/result.bin", b"artifact-v1\0bytes");
    let context = fixture.context();
    let artifacts = capture(
        &context,
        vec![PublicArtifact::with_digest(
            "out/result.bin",
            digest(b"artifact-v1\0bytes"),
        )],
    )
    .unwrap();
    let artifact = &artifacts[0];
    assert_eq!(artifact.context_id(), context.context_id());
    assert!(artifact.candidate_id().starts_with("sha256:"));
    assert_eq!(
        artifact.relative_path().and_then(|path| path.to_str()),
        Some("out/result.bin")
    );
    assert_eq!(artifact.sha256(), digest(b"artifact-v1\0bytes"));
    assert_eq!(artifact.byte_length(), 17);
    assert_eq!(artifact.bytes(), b"artifact-v1\0bytes");
    fs::write(fixture.root().join("out/result.bin"), b"tampered").unwrap();
    assert_eq!(artifact.bytes(), b"artifact-v1\0bytes");

    let json = artifact.to_canonical_json().unwrap();
    let value: serde_json::Value = serde_json::from_slice(&json).unwrap();
    assert_eq!(value["content_sha256"], artifact.sha256());
    assert_eq!(value["byte_length"], 17);
    assert_eq!(value["context_id"], context.context_id());
    assert_eq!(value["candidate_id"], artifact.candidate_id());
    assert_eq!(value["store_identity"], "immutable-memory-arc-v1");
    assert_eq!(value["resolver_identity"], "captured-artifact-sealed-v1");
    assert!(value.get("bytes").is_none());
    assert!(value.get("relative_path").is_none());
    let reference = artifact.artifact_ref();
    assert_eq!(reference.context_id(), context.context_id());
    assert_eq!(reference.candidate_id(), artifact.candidate_id());
    assert_eq!(reference.sha256(), artifact.sha256());
    assert_eq!(reference.byte_length(), artifact.byte_length());
    assert_eq!(&*artifact.resolve(&reference).unwrap(), artifact.bytes());
}

#[test]
fn same_length_ignored_artifact_mutation_produces_distinct_canonical_records() {
    let fixture = RepoFixture::new("artifact-ignored-mutation");
    fixture.write_file(".gitignore", b"out/ignored.bin\n");
    let context = fixture.context();
    fixture.write_file("out/ignored.bin", b"record-one");
    let first = capture(&context, vec![PublicArtifact::new("out/ignored.bin")])
        .unwrap()
        .remove(0);
    fixture.write_file("out/ignored.bin", b"record-two");
    let second = capture(&context, vec![PublicArtifact::new("out/ignored.bin")])
        .unwrap()
        .remove(0);

    assert_eq!(first.byte_length(), second.byte_length());
    assert_eq!(first.context_id(), second.context_id());
    assert_eq!(first.candidate_id(), second.candidate_id());
    assert_ne!(first.sha256(), second.sha256());
    assert_ne!(
        first.to_canonical_json().unwrap(),
        second.to_canonical_json().unwrap()
    );
}

#[test]
fn production_public_artifact_capture_requires_an_accepted_catalog_binding() {
    let fixture = RepoFixture::new("artifact-catalog-boundary");
    fixture.write_script("bin/noop", "exit 0");
    fixture.write_file("out/result", b"public");
    let error = read_command("bin/noop")
        .public_artifact(PublicArtifact::new("out/result"))
        .run(&fixture.context())
        .unwrap_err();
    assert!(error.contains("accepted typed-catalog binding"));
}

#[test]
fn missing_and_wrong_public_artifacts_fail_causally_in_test_substrate() {
    let fixture = RepoFixture::new("artifact-errors");
    fixture.write_file("out/result", b"actual");
    let context = fixture.context();
    let missing = capture(&context, vec![PublicArtifact::new("out/missing")]).unwrap_err();
    assert!(missing.contains("open failed"));
    let wrong = capture(
        &context,
        vec![PublicArtifact::with_digest("out/result", digest(b"wrong"))],
    )
    .unwrap_err();
    assert!(wrong.contains("required digest"));
}

#[cfg(unix)]
#[test]
fn symlink_and_hardlink_artifacts_are_rejected_in_test_substrate() {
    let fixture = RepoFixture::new("artifact-links");
    fixture.write_file("out/real", b"bytes");
    symlink("real", fixture.root().join("out/symlink")).unwrap();
    fs::hard_link(
        fixture.root().join("out/real"),
        fixture.root().join("out/hardlink"),
    )
    .unwrap();
    let context = fixture.context();
    for path in ["out/symlink", "out/hardlink", "out/real"] {
        let error = capture(&context, vec![PublicArtifact::new(path)]).unwrap_err();
        assert!(error.contains("open failed") || error.contains("hard-linked"));
    }
}

#[test]
fn artifact_swap_between_open_and_validation_is_rejected() {
    let fixture = RepoFixture::new("artifact-race");
    fixture.write_file("out/result", b"original");
    let context = fixture.context();
    let root = fixture.root().to_path_buf();
    set_test_artifact_pause_ms(100);
    let swapper = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(20));
        fs::rename(root.join("out/result"), root.join("out/original")).unwrap();
        fs::write(root.join("out/result"), b"replacement").unwrap();
    });
    let error = capture(&context, vec![PublicArtifact::new("out/result")]).unwrap_err();
    swapper.join().unwrap();
    assert!(
        error.contains("identity changed")
            || error.contains("content changed")
            || error.contains("context")
    );
}

#[test]
fn artifact_count_and_read_bytes_are_bounded_before_oversized_content() {
    let fixture = RepoFixture::new("artifact-bounds");
    fixture.write_script("bin/noop", "exit 0");
    let context = fixture.context();
    let mut excessive = read_command("bin/noop");
    for index in 0..129 {
        excessive = excessive.public_artifact(PublicArtifact::new(format!("out/{index}")));
    }
    assert!(
        excessive
            .run(&context)
            .unwrap_err()
            .contains("count exceeds")
    );

    let fixture = RepoFixture::new("artifact-byte-bounds");
    fixture.write_file(".gitignore", b"out/\n");
    let context = fixture.context();
    fixture.write_file("out/too-large", &vec![b'x'; 5 * MIB]);
    reset_test_descriptor_bytes_read();
    let error = capture(&context, vec![PublicArtifact::new("out/too-large")]).unwrap_err();
    assert!(error.contains("per-file byte bound"));
    assert_eq!(test_descriptor_bytes_read(), 0);

    fixture.write_file("out/a", &vec![b'a'; 3 * MIB]);
    fixture.write_file("out/b", &vec![b'b'; 3 * MIB]);
    fixture.write_file("out/c", &vec![b'c'; 3 * MIB]);
    reset_test_descriptor_bytes_read();
    let error = capture(
        &context,
        vec![
            PublicArtifact::new("out/a"),
            PublicArtifact::new("out/b"),
            PublicArtifact::new("out/c"),
        ],
    )
    .unwrap_err();
    assert!(error.contains("aggregate artifact bytes"));
    assert_eq!(test_descriptor_bytes_read(), 12 * MIB as u64);
}

#[test]
fn secret_artifact_channel_rejects_without_reading_or_echoing_identity() {
    let fixture = RepoFixture::new("artifact-secret");
    fixture.write_script("bin/noop", "exit 0");
    let canary = "tiny-secret-41";
    let digest = digest(canary.as_bytes());
    let error = read_command("bin/noop")
        .secret_artifact(SecretArtifact::new(
            format!("out/{canary}"),
            "BOUND_ARTIFACT",
        ))
        .run(&fixture.context())
        .unwrap_err();
    assert!(error.contains("secret artifact capture is unsupported"));
    assert!(!error.contains(canary));
    assert!(!error.contains(&digest));
    assert!(!fixture.root().join(format!("out/{canary}")).exists());
}
