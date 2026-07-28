#![cfg(target_os = "macos")]

use super::capture::CapturedRun;
use super::fixture::RepoFixture;
use super::secret_capture_fixture::digest;
use crate::context::{BuildRequest, LiveContext};
use sha2::{Digest, Sha256};

pub(super) fn context(fixture: &RepoFixture) -> LiveContext {
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

pub(super) fn assert_withheld_run(run: &CapturedRun, needles: &[&[u8]]) -> serde_json::Value {
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

pub(super) fn assert_public_run(
    run: &CapturedRun,
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
