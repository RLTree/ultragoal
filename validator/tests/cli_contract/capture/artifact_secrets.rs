use super::capture::{
    CommandSpec, PublicArg, PublicArtifact, SecretArg, SecretEnv, capture_spec_artifacts_for_test,
};
use super::fixture::RepoFixture;
use super::secret_capture_fixture::{
    assert_serialized_absent, assert_withheld, bound_context, digest,
};

#[cfg(unix)]
use std::ffi::OsString;
#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;

#[test]
fn full_secret_arg_artifact_is_safe_on_every_surface() {
    let fixture = RepoFixture::new("artifact-full-secret");
    let secret = "zzq7V5-full-artifact-secret";
    fixture.write_file("out/full.bin", secret.as_bytes());
    let context = bound_context(&fixture, &[("ARG_SECRET", "v1")], &[]);
    let spec = CommandSpec::catalog_read("artifact-full-secret", "true")
        .secret_arg(SecretArg::new(secret, "ARG_SECRET"))
        .public_artifact(PublicArtifact::with_digest(
            "out/full.bin",
            digest(secret.as_bytes()),
        ));

    let artifacts = capture_spec_artifacts_for_test(&context, &spec).unwrap();
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].context_id(), context.context_id());
    assert_withheld(&artifacts[0], &[secret.as_bytes()]);
}

#[test]
fn every_terminal_proper_secret_prefix_is_safe_on_every_surface() {
    let fixture = RepoFixture::new("artifact-secret-prefixes");
    let secret = "zzq7V5-bounded-artifact-secret";
    for cut in 1..secret.len() {
        let bytes = [b"public-before-".as_slice(), &secret.as_bytes()[..cut]].concat();
        fixture.write_file(&format!("out/prefix-{cut}.bin"), &bytes);
    }
    let context = bound_context(&fixture, &[("ARG_SECRET", "v1")], &[]);
    let mut spec = CommandSpec::catalog_read("artifact-secret-prefixes", "true")
        .secret_arg(SecretArg::new(secret, "ARG_SECRET"));
    for cut in 1..secret.len() {
        let bytes = [b"public-before-".as_slice(), &secret.as_bytes()[..cut]].concat();
        spec = spec.public_artifact(PublicArtifact::with_digest(
            format!("out/prefix-{cut}.bin"),
            digest(&bytes),
        ));
    }

    let artifacts = capture_spec_artifacts_for_test(&context, &spec).unwrap();
    assert_eq!(artifacts.len(), secret.len() - 1);
    for (cut, artifact) in (1..secret.len()).zip(&artifacts) {
        let bytes = [b"public-before-".as_slice(), &secret.as_bytes()[..cut]].concat();
        assert_eq!(artifact.relative_path(), None);
        assert_withheld(
            artifact,
            &[secret.as_bytes(), &secret.as_bytes()[..cut], &bytes],
        );
    }
}

#[test]
fn environment_only_secret_artifact_is_safe_on_every_surface() {
    let fixture = RepoFixture::new("artifact-environment-only-secret");
    let secret = "zzq7V5-environment-only-artifact-secret";
    let bytes = [b"public-before-".as_slice(), secret.as_bytes()].concat();
    fixture.write_file("out/environment.bin", &bytes);
    let context = bound_context(&fixture, &[("ENV_SECRET", "v1")], &[]);
    let spec = CommandSpec::catalog_read("artifact-environment-only-secret", "true")
        .secret_environment(SecretEnv::new("TOKEN", secret, "ENV_SECRET"))
        .public_artifact(PublicArtifact::with_digest(
            "out/environment.bin",
            digest(&bytes),
        ));

    let artifacts = capture_spec_artifacts_for_test(&context, &spec).unwrap();
    assert_eq!(artifacts.len(), 1);
    assert_withheld(&artifacts[0], &[secret.as_bytes(), &bytes]);
}

#[test]
fn overlapping_arg_and_env_secrets_are_safe_in_both_artifact_orders() {
    let fixture = RepoFixture::new("artifact-overlapping-secrets");
    let short = "zzq7";
    let long = "zzq7V5-overlapping-artifact-secret";
    let sensitive = [b"before-".as_slice(), long.as_bytes(), b"-after".as_slice()].concat();
    let public = b"ordinary-public-artifact";
    fixture.write_file("out/sensitive.bin", &sensitive);
    fixture.write_file("out/public.bin", public);
    let context = bound_context(&fixture, &[("ARG_SECRET", "v1"), ("ENV_SECRET", "v2")], &[]);

    for swap_secret_roles in [false, true] {
        for sensitive_first in [false, true] {
            let (argument, environment) = if swap_secret_roles {
                (short, long)
            } else {
                (long, short)
            };
            let mut spec = CommandSpec::catalog_read("artifact-overlap", "true")
                .secret_arg(SecretArg::new(argument, "ARG_SECRET"))
                .secret_environment(SecretEnv::new("TOKEN", environment, "ENV_SECRET"));
            let ordered = if sensitive_first {
                ["out/sensitive.bin", "out/public.bin"]
            } else {
                ["out/public.bin", "out/sensitive.bin"]
            };
            for path in ordered {
                let bytes = if path.ends_with("sensitive.bin") {
                    sensitive.as_slice()
                } else {
                    public.as_slice()
                };
                spec = spec.public_artifact(PublicArtifact::with_digest(path, digest(bytes)));
            }

            let artifacts = capture_spec_artifacts_for_test(&context, &spec).unwrap();
            assert!(
                artifacts
                    .iter()
                    .all(|artifact| artifact.relative_path().is_none())
            );
            for artifact in &artifacts {
                assert_eq!(artifact.context_id(), context.context_id());
                assert_withheld(
                    artifact,
                    &[&sensitive, public, long.as_bytes(), short.as_bytes()],
                );
            }
        }
    }
}

#[cfg(unix)]
#[test]
fn non_utf8_secret_and_terminal_prefix_are_safe_on_every_surface() {
    let fixture = RepoFixture::new("artifact-non-utf8-secret");
    let secret = vec![0xff, 0xfe, b'z', b'q', b'7', b'V', b'5'];
    let prefix = &secret[..secret.len() - 1];
    let bytes = [b"public-before-".as_slice(), prefix].concat();
    fixture.write_file("out/non-utf8.bin", &bytes);
    let context = bound_context(&fixture, &[("ARG_SECRET", "v1")], &[]);
    let spec = CommandSpec::catalog_read("artifact-non-utf8-secret", "true")
        .secret_arg(SecretArg::new(
            OsString::from_vec(secret.clone()),
            "ARG_SECRET",
        ))
        .public_artifact(PublicArtifact::with_digest(
            "out/non-utf8.bin",
            digest(&bytes),
        ));

    let artifacts = capture_spec_artifacts_for_test(&context, &spec).unwrap();
    assert_eq!(artifacts.len(), 1);
    assert_withheld(&artifacts[0], &[&secret, prefix, &bytes]);
}

#[cfg(target_os = "macos")]
#[test]
fn capture_run_artifacts_hide_full_secret_and_every_proper_prefix() {
    let fixture = RepoFixture::new("artifact-secret-run");
    let secret = "zzq7V5-canonical-artifact-secret";
    let public = b"ordinary-public-artifact";
    fixture.write_file("out/full.bin", secret.as_bytes());
    fixture.write_file("out/public.bin", public);
    for cut in 1..secret.len() {
        let bytes = [b"public-before-".as_slice(), &secret.as_bytes()[..cut]].concat();
        fixture.write_file(&format!("out/prefix-{cut}.bin"), &bytes);
    }
    let context = bound_context(
        &fixture,
        &[("ARG_SECRET", "v1")],
        &["sandbox-exec", "printf"],
    );

    for public_first in [false, true] {
        let mut paths = (1..secret.len())
            .map(|cut| format!("out/prefix-{cut}.bin"))
            .collect::<Vec<_>>();
        paths.push("out/full.bin".to_owned());
        if public_first {
            paths.insert(0, "out/public.bin".to_owned());
        } else {
            paths.push("out/public.bin".to_owned());
        }
        let mut spec = CommandSpec::catalog_read("artifact-secret-run", "printf")
            .public_arg(PublicArg::new("%s"))
            .secret_arg(SecretArg::new(secret, "ARG_SECRET"));
        for path in &paths {
            let bytes = if path == "out/full.bin" {
                secret.as_bytes().to_vec()
            } else if path == "out/public.bin" {
                public.to_vec()
            } else {
                let cut = path
                    .trim_start_matches("out/prefix-")
                    .trim_end_matches(".bin")
                    .parse::<usize>()
                    .unwrap();
                [b"public-before-".as_slice(), &secret.as_bytes()[..cut]].concat()
            };
            spec = spec.public_artifact(PublicArtifact::with_digest(path, digest(&bytes)));
        }

        let run = spec.run(&context).unwrap();
        assert_eq!(run.artifacts().len(), paths.len());
        for (path, artifact) in paths.iter().zip(run.artifacts()) {
            assert_eq!(artifact.context_id(), run.observed_context_id());
            assert_eq!(artifact.candidate_id(), run.candidate_id());
            assert_eq!(artifact.relative_path(), None);
            if path.ends_with("full.bin") {
                assert_withheld(artifact, &[secret.as_bytes()]);
            } else if path.ends_with("public.bin") {
                assert_withheld(artifact, &[secret.as_bytes(), public]);
            } else {
                let cut = path
                    .trim_start_matches("out/prefix-")
                    .trim_end_matches(".bin")
                    .parse::<usize>()
                    .unwrap();
                let bytes = [b"public-before-".as_slice(), &secret.as_bytes()[..cut]].concat();
                assert_withheld(
                    artifact,
                    &[secret.as_bytes(), &secret.as_bytes()[..cut], &bytes],
                );
            }
        }
        let json = run.to_canonical_json().unwrap();
        let debug = format!("{run:?}");
        let mut needle_storage = vec![secret.as_bytes().to_vec()];
        for cut in 1..secret.len() {
            needle_storage.push(secret.as_bytes()[..cut].to_vec());
            needle_storage.push([b"public-before-".as_slice(), &secret.as_bytes()[..cut]].concat());
        }
        let needles = needle_storage.iter().map(Vec::as_slice).collect::<Vec<_>>();
        assert_serialized_absent(&json, &needles);
        assert_serialized_absent(debug.as_bytes(), &needles);
    }
}
