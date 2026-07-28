use super::capture::{
    CommandSpec, PublicArtifact, SecretArg, SecretArtifact, SecretEnv,
    capture_spec_artifacts_for_test, reset_test_descriptor_bytes_read,
    reset_test_file_open_attempts, test_descriptor_bytes_read, test_file_open_attempts,
};
use super::fixture::RepoFixture;
use super::secret_capture_fixture::{bound_context, digest};

fn assert_no_file_access() {
    assert_eq!(test_file_open_attempts(), 0);
    assert_eq!(test_descriptor_bytes_read(), 0);
}

fn reset_file_access() {
    reset_test_file_open_attempts();
    reset_test_descriptor_bytes_read();
}

#[test]
fn public_and_secret_artifact_roles_fail_closed_in_both_orders() {
    let fixture = RepoFixture::new("artifact-role-order");
    fixture.write_file("out/public.bin", b"public");
    let context = bound_context(&fixture, &[], &[]);

    for secret_first in [false, true] {
        let public = PublicArtifact::new("out/public.bin");
        let secret = SecretArtifact::new("out/secret.bin", "BOUND_ARTIFACT");
        let spec = if secret_first {
            CommandSpec::catalog_read("artifact-role-order", "true")
                .secret_artifact(secret)
                .public_artifact(public)
        } else {
            CommandSpec::catalog_read("artifact-role-order", "true")
                .public_artifact(public)
                .secret_artifact(secret)
        };
        reset_file_access();
        let error = capture_spec_artifacts_for_test(&context, &spec).unwrap_err();
        assert!(error.contains("secret artifact capture is unsupported"));
        assert_no_file_access();
    }
}

#[test]
fn artifact_path_label_cannot_reclassify_a_secret_arg_as_public() {
    let fixture = RepoFixture::new("artifact-secret-path-label");
    let secret = "zzq7V5-artifact-path-secret";
    let context = bound_context(&fixture, &[("ARG_SECRET", "v1")], &[]);
    let spec = CommandSpec::catalog_read("artifact-secret-path-label", "true")
        .secret_arg(SecretArg::new(secret, "ARG_SECRET"))
        .public_artifact(PublicArtifact::new(format!("out/{secret}.bin")));

    reset_file_access();
    let error = capture_spec_artifacts_for_test(&context, &spec).unwrap_err();
    assert!(error.contains("outside an explicit secret channel"));
    assert!(!error.contains(secret));
    assert!(!error.contains(&digest(secret.as_bytes())));
    assert_no_file_access();
}

#[test]
fn artifact_digest_label_cannot_reclassify_a_secret_env_as_public() {
    let fixture = RepoFixture::new("artifact-secret-digest-label");
    let secret_digest = digest(b"artifact-label-canary");
    let context = bound_context(&fixture, &[("ENV_SECRET", "v1")], &[]);
    let spec = CommandSpec::catalog_read("artifact-secret-digest-label", "true")
        .secret_environment(SecretEnv::new(
            "TOKEN",
            secret_digest.as_str(),
            "ENV_SECRET",
        ))
        .public_artifact(PublicArtifact::with_digest(
            "out/not-opened.bin",
            &secret_digest,
        ));

    reset_file_access();
    let error = capture_spec_artifacts_for_test(&context, &spec).unwrap_err();
    assert!(error.contains("outside an explicit secret channel"));
    assert!(!error.contains(&secret_digest));
    assert_no_file_access();
}

#[test]
fn artifact_path_label_cannot_reclassify_a_secret_env_as_public() {
    let fixture = RepoFixture::new("artifact-env-path-label");
    let secret = "zzq7V5-artifact-env-path-secret";
    let context = bound_context(&fixture, &[("ENV_SECRET", "v1")], &[]);
    let spec = CommandSpec::catalog_read("artifact-env-path-label", "true")
        .secret_environment(SecretEnv::new("TOKEN", secret, "ENV_SECRET"))
        .public_artifact(PublicArtifact::new(format!("out/{secret}.bin")));

    reset_file_access();
    let error = capture_spec_artifacts_for_test(&context, &spec).unwrap_err();
    assert!(error.contains("outside an explicit secret channel"));
    assert!(!error.contains(secret));
    assert!(!error.contains(&digest(secret.as_bytes())));
    assert_no_file_access();
}

#[test]
fn artifact_digest_label_cannot_reclassify_a_secret_arg_as_public() {
    let fixture = RepoFixture::new("artifact-arg-digest-label");
    let secret_digest = digest(b"artifact-arg-label-canary");
    let context = bound_context(&fixture, &[("ARG_SECRET", "v1")], &[]);
    let spec = CommandSpec::catalog_read("artifact-arg-digest-label", "true")
        .secret_arg(SecretArg::new(&secret_digest, "ARG_SECRET"))
        .public_artifact(PublicArtifact::with_digest(
            "out/not-opened.bin",
            &secret_digest,
        ));

    reset_file_access();
    let error = capture_spec_artifacts_for_test(&context, &spec).unwrap_err();
    assert!(error.contains("outside an explicit secret channel"));
    assert!(!error.contains(&secret_digest));
    assert_no_file_access();
}

#[test]
fn empty_secret_is_rejected_before_artifact_access() {
    let fixture = RepoFixture::new("artifact-empty-secret");
    fixture.write_file("out/not-opened.bin", b"public");
    let context = bound_context(&fixture, &[("ARG_SECRET", "v1")], &[]);
    let spec = CommandSpec::catalog_read("artifact-empty-secret", "true")
        .secret_arg(SecretArg::new("", "ARG_SECRET"))
        .public_artifact(PublicArtifact::new("out/not-opened.bin"));

    reset_file_access();
    let error = capture_spec_artifacts_for_test(&context, &spec).unwrap_err();
    assert!(error.contains("unsafe length"));
    assert_no_file_access();
}
