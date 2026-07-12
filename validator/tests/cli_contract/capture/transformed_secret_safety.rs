#[cfg(target_os = "macos")]
mod macos {
    use super::super::artifact_secret_support::digest;
    use super::super::capture::{CommandSpec, PublicArg, SecretArg, SecretEnv};
    use super::super::fixture::RepoFixture;
    use crate::context::{BuildRequest, LiveContext};
    use sha2::{Digest, Sha256};
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    fn context(fixture: &RepoFixture) -> LiveContext {
        LiveContext::build(
            BuildRequest::new(fixture.root())
                .expect_repository_root(fixture.root())
                .expect_worktree_root(fixture.root())
                .bind_secret_source("ARG_SECRET", "v1")
                .bind_secret_source("SECOND_SECRET", "v1")
                .bind_secret_source("ENV_SECRET", "v1")
                .probe_tool("sandbox-exec")
                .probe_tool("printf"),
        )
        .unwrap()
    }

    fn assert_absent(surface: &[u8], needles: &[&[u8]]) {
        let text = String::from_utf8_lossy(surface);
        for needle in needles.iter().copied().filter(|needle| !needle.is_empty()) {
            assert!(!surface.windows(needle.len()).any(|seen| seen == needle));
            let raw = format!("sha256:{:x}", Sha256::digest(needle));
            let bare = raw.strip_prefix("sha256:").unwrap();
            assert!(!text.contains(&raw) && !text.contains(bare));
            let numeric = format!(
                "[{}]",
                needle
                    .iter()
                    .map(u8::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            );
            assert!(!text.contains(&numeric));
            if needle.len() >= 4 {
                let encoded = needle
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>();
                assert!(
                    !text.contains(&encoded),
                    "hex-encoded secret-derived bytes were observable"
                );
            }
        }
    }

    fn assert_withheld_run(
        run: &super::super::capture::CapturedRun,
        needles: &[&[u8]],
    ) -> serde_json::Value {
        assert!(run.stdout_bytes().is_empty());
        assert!(run.stderr_bytes().is_empty());
        let json = run.to_canonical_json().unwrap();
        let debug = format!("{run:?}");
        assert_absent(&json, needles);
        assert_absent(debug.as_bytes(), needles);
        let value: serde_json::Value = serde_json::from_slice(&json).unwrap();
        assert_eq!(
            value["invocation_metadata_disposition"],
            "withheld-secret-bearing-invocation"
        );
        assert!(value.get("cwd_path_hex").is_none());
        assert_eq!(
            value["termination"]["kind"],
            "withheld-secret-bearing-invocation"
        );
        assert_eq!(run.monotonic_duration_ns(), 0);
        assert!(!run.interrupted());
        for collection in ["arguments", "environment"] {
            assert!(value[collection].as_array().unwrap().iter().all(|record| {
                record.as_object().unwrap().len() == 1
                    && record["channel"] == "withheld-secret-bearing-invocation"
            }));
        }
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
            assert_eq!(value[stream]["captured_sha256"], digest(&[]));
            assert_eq!(value[stream]["truncated"], true);
            assert_eq!(value[stream]["observation_limit_exceeded"], false);
            assert_eq!(value[stream]["captured_byte_length_is_lower_bound"], false);
        }
        value
    }

    fn assert_public_run(
        run: &super::super::capture::CapturedRun,
        stdout: &[u8],
        stderr_contains: &[u8],
        code: i64,
    ) {
        assert_eq!(run.stdout_bytes(), stdout);
        assert!(
            stderr_contains.is_empty()
                || run
                    .stderr_bytes()
                    .windows(stderr_contains.len())
                    .any(|seen| seen == stderr_contains)
        );
        let value: serde_json::Value =
            serde_json::from_slice(&run.to_canonical_json().unwrap()).unwrap();
        assert_eq!(value["termination"]["kind"], "exited");
        assert_eq!(value["termination"]["code"], code);
        for stream in ["stdout", "stderr"] {
            assert_eq!(value[stream]["content_disposition"], "public");
        }
    }

    #[test]
    fn raw_hex_decimal_case_and_affix_transformations_are_withheld() {
        let fixture = RepoFixture::new("transformed-secret-runtime");
        let context = context(&fixture);
        let cases = [
            ("raw", "%s", "123456", "123456"),
            ("hex", "%x", "123456", "1e240"),
            ("decimal-normalization", "%d", "+123456", "123456"),
            ("upper-case-encoding", "%X", "11259375", "ABCDEF"),
            (
                "prefix-suffix",
                "prefix-%x-suffix",
                "123456",
                "prefix-1e240-suffix",
            ),
        ];
        std::thread::scope(|scope| {
            for (id, format, secret, transformed) in cases {
                let context = &context;
                scope.spawn(move || {
                    let public = CommandSpec::catalog_read(format!("{id}-public"), "printf")
                        .public_arg(PublicArg::new(format))
                        .public_arg(PublicArg::new(secret))
                        .run(context)
                        .unwrap();
                    assert_public_run(&public, transformed.as_bytes(), b"", 0);
                    let run = CommandSpec::catalog_read(id, "printf")
                        .public_arg(PublicArg::new(format))
                        .secret_arg(SecretArg::new(secret, "ARG_SECRET"))
                        .run(context)
                        .unwrap();
                    assert_withheld_run(&run, &[secret.as_bytes(), transformed.as_bytes()]);
                });
            }
        });
    }

    #[test]
    fn failed_two_stream_process_withholds_status_and_both_streams() {
        let fixture = RepoFixture::new("transformed-secret-failed-process");
        let context = context(&fixture);
        let first = "123456";
        let second = "badvalue";
        let public = CommandSpec::catalog_read("failed-two-stream-public", "printf")
            .public_arg(PublicArg::new("%x %d"))
            .public_arg(PublicArg::new(first))
            .public_arg(PublicArg::new(second))
            .run(&context)
            .unwrap();
        assert_public_run(&public, b"1e240 0", b"badvalue", 1);
        let run = CommandSpec::catalog_read("failed-two-stream", "printf")
            .public_arg(PublicArg::new("%x %d"))
            .secret_arg(SecretArg::new(first, "ARG_SECRET"))
            .secret_arg(SecretArg::new(second, "SECOND_SECRET"))
            .run(&context)
            .unwrap();
        let value = assert_withheld_run(
            &run,
            &[
                first.as_bytes(),
                second.as_bytes(),
                b"1e240",
                b"expected numeric value",
            ],
        );
        assert_eq!(
            value["termination"]["kind"],
            "withheld-secret-bearing-invocation"
        );
    }

    #[test]
    fn environment_non_utf8_overlap_and_truncation_remain_fail_closed() {
        let fixture = RepoFixture::new("transformed-secret-boundaries");
        let context = context(&fixture);
        std::thread::scope(|scope| {
            let context = &context;
            scope.spawn(move || {
                let secret = "environment-secret-123456";
                let public = CommandSpec::catalog_read("environment-public", "printf")
                    .public_arg(PublicArg::new("ordinary-public-output"))
                    .run(context)
                    .unwrap();
                assert_public_run(&public, b"ordinary-public-output", b"", 0);
                let run = CommandSpec::catalog_read("environment-taint", "printf")
                    .public_arg(PublicArg::new("ordinary-public-output"))
                    .secret_environment(SecretEnv::new("TOKEN", secret, "ENV_SECRET"))
                    .run(context)
                    .unwrap();
                assert_withheld_run(&run, &[secret.as_bytes(), b"ordinary-public-output"]);
            });
            scope.spawn(move || {
                let bytes = vec![0xff, 0xfe, b'A', b'B'];
                let public = CommandSpec::catalog_read("non-utf8-public", "printf")
                    .public_arg(PublicArg::new("%s"))
                    .public_arg(PublicArg::new(OsString::from_vec(bytes.clone())))
                    .run(context)
                    .unwrap();
                assert_public_run(&public, &bytes, b"", 0);
                let run = CommandSpec::catalog_read("non-utf8-taint", "printf")
                    .public_arg(PublicArg::new("%s"))
                    .secret_arg(SecretArg::new(
                        OsString::from_vec(bytes.clone()),
                        "ARG_SECRET",
                    ))
                    .run(context)
                    .unwrap();
                assert_withheld_run(&run, &[&bytes]);
            });

            scope.spawn(move || {
                let (short, long) = ("123456", "1234567");
                let public = CommandSpec::catalog_read("overlapping-public", "printf")
                    .public_arg(PublicArg::new("%x %x"))
                    .public_arg(PublicArg::new(short))
                    .public_arg(PublicArg::new(long))
                    .run(context)
                    .unwrap();
                assert_public_run(&public, b"1e240 12d687", b"", 0);
                let run = CommandSpec::catalog_read("overlapping-taint", "printf")
                    .public_arg(PublicArg::new("%x %x"))
                    .secret_arg(SecretArg::new(short, "ARG_SECRET"))
                    .secret_arg(SecretArg::new(long, "SECOND_SECRET"))
                    .run(context)
                    .unwrap();
                assert_withheld_run(
                    &run,
                    &[short.as_bytes(), long.as_bytes(), b"1e240", b"12d687"],
                );
            });

            scope.spawn(move || {
                let secret = "123456";
                let public = CommandSpec::catalog_read("truncated-public", "printf")
                    .public_arg(PublicArg::new("prefix-%x-suffix"))
                    .public_arg(PublicArg::new(secret))
                    .output_limit(1)
                    .observed_output_limit(3)
                    .run(context)
                    .unwrap();
                let public_value: serde_json::Value =
                    serde_json::from_slice(&public.to_canonical_json().unwrap()).unwrap();
                assert_eq!(public.stdout_bytes(), b"p");
                assert_eq!(public_value["termination"]["kind"], "output-limit");
                let run = CommandSpec::catalog_read("truncated-transform", "printf")
                    .public_arg(PublicArg::new("prefix-%x-suffix"))
                    .secret_arg(SecretArg::new(secret, "ARG_SECRET"))
                    .output_limit(1)
                    .observed_output_limit(3)
                    .run(context)
                    .unwrap();
                let value = assert_withheld_run(
                    &run,
                    &[secret.as_bytes(), b"1e240", b"prefix-1e240-suffix"],
                );
                assert_eq!(
                    value["termination"]["kind"],
                    "withheld-secret-bearing-invocation"
                );
                assert_eq!(value["stdout"]["observation_limit_exceeded"], false);
            });
        });
    }

    #[test]
    fn empty_secret_rejects_before_launch_without_echo_or_digest() {
        let fixture = RepoFixture::new("transformed-secret-empty");
        let context = context(&fixture);
        let error = CommandSpec::catalog_read("empty-secret", "printf")
            .public_arg(PublicArg::new("%s"))
            .secret_arg(SecretArg::new("", "ARG_SECRET"))
            .run(&context)
            .unwrap_err();
        assert!(error.contains("unsafe length"));
        assert!(!error.contains(&digest(&[])));
    }
}
