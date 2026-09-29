use super::capture::{
    CommandSpec, PublicArg, PublicArtifact, SecretArg, capture_spec_artifacts_for_test,
    reset_test_descriptor_bytes_read, reset_test_file_open_attempts, test_descriptor_bytes_read,
    test_file_open_attempts,
};
use super::fixture::RepoFixture;
use super::secret_capture_fixture::{
    assert_public, assert_serialized_absent, assert_withheld, bound_context, digest,
};

#[cfg(unix)]
use std::ffi::CString;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;

fn reset_access() {
    reset_test_file_open_attempts();
    reset_test_descriptor_bytes_read();
}

#[cfg(target_os = "macos")]
#[test]
fn transformed_artifact_content_is_absent_from_the_canonical_run() {
    let fixture = RepoFixture::new("transformed-secret-artifact-run");
    let secret = "48879";
    let transformed = b"beef";
    fixture.write_file("out/beef.bin", transformed);
    let context = bound_context(
        &fixture,
        &[("ARG_SECRET", "v1")],
        &["sandbox-exec", "printf"],
    );
    let run = CommandSpec::catalog_read("transformed-artifact-run", "printf")
        .public_arg(PublicArg::new("%x"))
        .secret_arg(SecretArg::new(secret, "ARG_SECRET"))
        .public_artifact(PublicArtifact::with_digest(
            "out/beef.bin",
            digest(transformed),
        ))
        .run(&context)
        .unwrap();
    assert!(run.stdout_bytes().is_empty());
    assert_eq!(run.artifacts().len(), 1);
    assert_withheld(&run.artifacts()[0], &[secret.as_bytes(), transformed]);
    let json = run.to_canonical_json().unwrap();
    let debug = format!("{run:?}");
    assert_serialized_absent(&json, &[secret.as_bytes(), transformed]);
    assert_serialized_absent(debug.as_bytes(), &[secret.as_bytes(), transformed]);
}

#[cfg(unix)]
#[test]
fn secret_bearing_artifacts_are_classified_before_any_open_or_read() {
    let fixture = RepoFixture::new("transformed-secret-artifacts");
    let secret = "+48879";
    let rows: [(&str, &[u8]); 6] = [
        ("out/raw.bin", b"+48879"),
        ("out/decimal.bin", b"48879"),
        ("out/beef.bin", b"beef"),
        ("out/upper.bin", b"BEEF"),
        ("out/encoded.bin", b"KzQ4ODc5"),
        ("out/affix.bin", b"prefix-beef-suffix"),
    ];
    for (path, bytes) in rows {
        fixture.write_file(path, bytes);
    }
    let fifo = fixture.root().join("out/secret-output.fifo");
    let fifo_c = CString::new(fifo.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo_c.as_ptr(), 0o600) }, 0);
    let context = bound_context(&fixture, &[("ARG_SECRET", "v1")], &[]);

    let mut public_spec = CommandSpec::catalog_read("transformed-artifact-public", "true");
    for (path, bytes) in rows {
        public_spec = public_spec.public_artifact(PublicArtifact::with_digest(path, digest(bytes)));
    }
    reset_access();
    let public = capture_spec_artifacts_for_test(&context, &public_spec).unwrap();
    assert!(test_file_open_attempts() > 0);
    assert!(test_descriptor_bytes_read() > 0);
    for ((_, bytes), artifact) in rows.iter().zip(&public) {
        assert_public(artifact, bytes);
    }

    for reverse in [false, true] {
        let mut paths = rows.iter().map(|(path, _)| *path).collect::<Vec<_>>();
        paths.extend(["out/secret-output.fifo", "out/missing.bin"]);
        if reverse {
            paths.reverse();
        }
        let mut spec = CommandSpec::catalog_read("transformed-artifact-secret", "true")
            .secret_arg(SecretArg::new(secret, "ARG_SECRET"));
        for path in &paths {
            let expected = rows
                .iter()
                .find(|(candidate, _)| candidate == path)
                .map(|(_, bytes)| digest(bytes));
            spec = spec.public_artifact(match expected {
                Some(expected) => PublicArtifact::with_digest(path, expected),
                None => PublicArtifact::new(path),
            });
        }

        reset_access();
        let artifacts = capture_spec_artifacts_for_test(&context, &spec).unwrap();
        assert_eq!(test_file_open_attempts(), 0);
        assert_eq!(test_descriptor_bytes_read(), 0);
        assert_eq!(artifacts.len(), paths.len());
        for (path, artifact) in paths.iter().zip(&artifacts) {
            assert_eq!(artifact.relative_path(), None);
            let mut needles = vec![secret.as_bytes()];
            if let Some((_, bytes)) = rows.iter().find(|(candidate, _)| candidate == path) {
                needles.push(*bytes);
            }
            assert_withheld(artifact, &needles);
        }
    }
}
