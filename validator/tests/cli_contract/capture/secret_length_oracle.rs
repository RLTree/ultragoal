#[cfg(target_os = "macos")]
mod macos {
    use super::super::capture::{
        CommandSpec, PublicArg, PublicArtifact, SecretArg, capture_spec_artifacts_for_test,
        reset_test_descriptor_bytes_read, reset_test_file_open_attempts,
        test_descriptor_bytes_read, test_file_open_attempts,
    };
    use super::super::fixture::RepoFixture;
    use crate::context::{BuildRequest, LiveContext};
    use sha2::{Digest, Sha256};
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::time::Duration;

    fn context(fixture: &RepoFixture) -> LiveContext {
        LiveContext::build(
            BuildRequest::new(fixture.root())
                .expect_repository_root(fixture.root())
                .expect_worktree_root(fixture.root())
                .bind_secret_source("SECRET", "v1")
                .bind_secret_source("SECOND_SECRET", "v1")
                .probe_tool("sandbox-exec")
                .probe_tool("false")
                .probe_tool("printf")
                .probe_tool("sleep"),
        )
        .unwrap()
    }

    fn empty_digest() -> String {
        format!("sha256:{:x}", Sha256::digest([]))
    }

    fn stable_surfaces(run: &super::super::capture::CapturedRun) -> (Vec<u8>, String) {
        assert!(run.stdout_bytes().is_empty());
        assert!(run.stderr_bytes().is_empty());
        assert_eq!(run.monotonic_duration_ns(), 0);
        assert!(!run.interrupted());
        let json = run.to_canonical_json().unwrap();
        let debug = format!("{run:?}");
        let value: serde_json::Value = serde_json::from_slice(&json).unwrap();
        assert_eq!(
            value["termination"],
            serde_json::json!({"kind": "withheld-secret-bearing-invocation"})
        );
        for stream in ["stdout", "stderr"] {
            assert_eq!(
                value[stream]["content_disposition"],
                "withheld-secret-bearing-invocation"
            );
            assert_eq!(
                value[stream]["encoding"],
                "withheld-secret-bearing-invocation-v1"
            );
            assert_eq!(value[stream]["retained_bytes"], serde_json::json!([]));
            assert_eq!(value[stream]["captured_byte_length"], 0);
            assert_eq!(value[stream]["captured_sha256"], empty_digest());
            assert_eq!(value[stream]["truncated"], true);
            assert_eq!(value[stream]["observation_limit_exceeded"], false);
            assert_eq!(value[stream]["captured_byte_length_is_lower_bound"], false);
        }
        (json, debug)
    }

    fn assert_identical(rows: Vec<(Vec<u8>, String)>) {
        let first = rows.first().unwrap();
        for row in &rows[1..] {
            assert_eq!(row.0, first.0);
            assert_eq!(row.1, first.1);
        }
    }

    #[test]
    fn identical_secret_percent_s_is_invariant_across_binary_searched_limits() {
        let fixture = RepoFixture::new("secret-length-percent-s");
        let context = context(&fixture);
        let secret = "S".repeat(64);
        let limits = [1, 2, 4, 8, 16, 32, 63, 64, 65, 128];
        let rows = std::thread::scope(|scope| {
            limits
                .into_iter()
                .map(|limit| {
                    let context = &context;
                    let secret = &secret;
                    scope.spawn(move || {
                        let run = CommandSpec::catalog_read("secret-length-percent-s", "printf")
                            .public_arg(PublicArg::new("%s"))
                            .secret_arg(SecretArg::new(secret.as_str(), "SECRET"))
                            .output_limit(limit)
                            .observed_output_limit(limit)
                            .run(context)
                            .unwrap();
                        stable_surfaces(&run)
                    })
                })
                .collect::<Vec<_>>()
                .into_iter()
                .map(|handle| handle.join().unwrap())
                .collect()
        });
        assert_identical(rows);
    }

    #[test]
    fn numeric_width_and_non_utf8_oracles_have_one_projection() {
        let fixture = RepoFixture::new("secret-length-width-non-utf8");
        let context = context(&fixture);
        let width_rows = ["0004", "0016", "0064", "1024"]
            .into_iter()
            .map(|width| {
                let run = CommandSpec::catalog_read("secret-width-oracle", "printf")
                    .public_arg(PublicArg::new("%*s"))
                    .secret_arg(SecretArg::new(width, "SECRET"))
                    .public_arg(PublicArg::new("x"))
                    .output_limit(32)
                    .observed_output_limit(32)
                    .run(&context)
                    .unwrap();
                stable_surfaces(&run)
            })
            .collect();
        assert_identical(width_rows);

        let non_utf8 = OsString::from_vec(vec![0xff, 0xfe, b'A', b'B', b'C', b'D']);
        let rows = [1, 6, 64]
            .into_iter()
            .map(|limit| {
                let run = CommandSpec::catalog_read("secret-non-utf8-oracle", "printf")
                    .public_arg(PublicArg::new("%s"))
                    .secret_arg(SecretArg::new(non_utf8.clone(), "SECRET"))
                    .output_limit(limit)
                    .observed_output_limit(limit)
                    .run(&context)
                    .unwrap();
                stable_surfaces(&run)
            })
            .collect();
        assert_identical(rows);
    }

    #[test]
    fn stderr_and_joint_crossings_do_not_publish_limit_or_exit_state() {
        let fixture = RepoFixture::new("secret-length-stream-crossings");
        let context = context(&fixture);
        for joint in [false, true] {
            let rows = [1, 8, 64, 256]
                .into_iter()
                .map(|limit| {
                    let spec = if joint {
                        CommandSpec::catalog_read("secret-joint-oracle", "printf")
                            .public_arg(PublicArg::new("%*s %d"))
                            .secret_arg(SecretArg::new("0064", "SECRET"))
                            .public_arg(PublicArg::new("x"))
                            .secret_arg(SecretArg::new("bad-secret", "SECOND_SECRET"))
                    } else {
                        CommandSpec::catalog_read("secret-stderr-oracle", "printf")
                            .public_arg(PublicArg::new("%d"))
                            .secret_arg(SecretArg::new("bad-secret", "SECRET"))
                    };
                    stable_surfaces(
                        &spec
                            .output_limit(limit)
                            .observed_output_limit(limit)
                            .run(&context)
                            .unwrap(),
                    )
                })
                .collect();
            assert_identical(rows);
        }
    }

    #[test]
    fn timeout_nonzero_and_public_threshold_controls_remain_honest() {
        let fixture = RepoFixture::new("secret-length-control-causes");
        let context = context(&fixture);
        let timed_out = CommandSpec::catalog_read("secret-timeout-oracle", "sleep")
            .secret_arg(SecretArg::new("0.05", "SECRET"))
            .timeout(Duration::from_millis(1))
            .run(&context)
            .unwrap();
        let exited = CommandSpec::catalog_read("secret-timeout-oracle", "sleep")
            .secret_arg(SecretArg::new("0.05", "SECRET"))
            .timeout(Duration::from_millis(250))
            .run(&context)
            .unwrap();
        let interrupted = CommandSpec::catalog_read("secret-timeout-oracle", "sleep")
            .secret_arg(SecretArg::new("0.05", "SECRET"))
            .with_interrupt_flag(Arc::new(AtomicBool::new(true)))
            .run(&context)
            .unwrap();
        assert_identical(vec![
            stable_surfaces(&timed_out),
            stable_surfaces(&exited),
            stable_surfaces(&interrupted),
        ]);

        let nonzero = CommandSpec::catalog_read("secret-nonzero", "false")
            .secret_arg(SecretArg::new("ignored-secret", "SECRET"))
            .run(&context)
            .unwrap();
        stable_surfaces(&nonzero);

        let public = "P".repeat(64);
        let exact = CommandSpec::catalog_read("public-length-control", "printf")
            .public_arg(PublicArg::new("%s"))
            .public_arg(PublicArg::new(public.as_str()))
            .output_limit(128)
            .observed_output_limit(128)
            .run(&context)
            .unwrap();
        assert_eq!(exact.stdout_bytes(), public.as_bytes());
        let exact_json: serde_json::Value =
            serde_json::from_slice(&exact.to_canonical_json().unwrap()).unwrap();
        assert_eq!(exact_json["termination"]["kind"], "exited");
        assert_eq!(exact_json["stdout"]["captured_byte_length"], 64);
        assert_eq!(exact_json["stdout"]["observation_limit_exceeded"], false);

        let bounded = CommandSpec::catalog_read("public-length-control", "printf")
            .public_arg(PublicArg::new("%s"))
            .public_arg(PublicArg::new(public))
            .output_limit(16)
            .observed_output_limit(16)
            .run(&context)
            .unwrap();
        assert_eq!(bounded.stdout_bytes().len(), 16);
        let bounded_json: serde_json::Value =
            serde_json::from_slice(&bounded.to_canonical_json().unwrap()).unwrap();
        assert_eq!(bounded_json["termination"]["kind"], "output-limit");
        assert_eq!(bounded_json["stdout"]["observation_limit_exceeded"], true);
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
}
