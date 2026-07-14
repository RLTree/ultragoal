use super::capture::{CommandSpec, PublicArg, SecretArg};
use super::fixture::RepoFixture;
use crate::context::{BuildRequest, LiveContext};
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;

fn requested_interrupt() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(true))
}

#[cfg(target_os = "macos")]
fn catalog_context(fixture: &RepoFixture, capability: &str) -> LiveContext {
    LiveContext::build(
        BuildRequest::new(fixture.root())
            .expect_repository_root(fixture.root())
            .expect_worktree_root(fixture.root())
            .probe_tool("sandbox-exec")
            .probe_tool(capability),
    )
    .unwrap()
}

#[cfg(target_os = "macos")]
#[test]
fn catalog_bound_native_read_executes_and_captures_non_utf8_without_writes() {
    let fixture = RepoFixture::new("native-read");
    let context = catalog_context(&fixture, "printf");
    let before = context.context_id().to_owned();
    let non_utf8 = OsString::from_vec(vec![0xff, 0xfe, b'A']);
    let run = CommandSpec::catalog_read("inspect-native-probe", "printf")
        .public_arg(PublicArg::new("%s"))
        .public_arg(PublicArg::new(non_utf8.clone()))
        .run(&context)
        .unwrap();
    assert_eq!(run.context_id(), before);
    assert_eq!(run.observed_context_id(), before);
    assert_eq!(run.stdout_bytes(), non_utf8.as_encoded_bytes());
    assert!(run.stderr_bytes().is_empty());
    let value: serde_json::Value =
        serde_json::from_slice(&run.to_canonical_json().unwrap()).unwrap();
    assert_eq!(value["schema_version"], "CapturedRun-v1");
    assert_eq!(value["tool_id"], "HCT-CAPTURE");
    assert_eq!(value["command_id"], "inspect-native-probe");
    assert_eq!(value["termination"]["kind"], "exited");
    assert_eq!(value["termination"]["code"], 0);
    assert_eq!(value["enforcement"]["network_allowed"], false);
    assert_eq!(value["enforcement"]["external_write_allowed"], false);
}

#[cfg(target_os = "macos")]
#[test]
fn catalog_bound_capture_redacts_secrets_and_bounds_output_timeout_and_interrupt() {
    let fixture = RepoFixture::new("native-controls");
    let secret = "secret-native-canary-779";
    let context = LiveContext::build(
        BuildRequest::new(fixture.root())
            .expect_repository_root(fixture.root())
            .expect_worktree_root(fixture.root())
            .bind_secret_source("CAPTURE_SECRET", "v1")
            .probe_tool("sandbox-exec")
            .probe_tool("printf")
            .probe_tool("yes")
            .probe_tool("sleep"),
    )
    .unwrap();
    let redacted = CommandSpec::catalog_read("redaction-probe", "printf")
        .public_arg(PublicArg::new("%s"))
        .secret_arg(SecretArg::new(secret, "CAPTURE_SECRET"))
        .run(&context)
        .unwrap();
    assert!(redacted.stdout_bytes().is_empty());
    assert!(redacted.stderr_bytes().is_empty());
    let json = redacted.to_canonical_json().unwrap();
    assert!(
        !json
            .windows(secret.len())
            .any(|bytes| bytes == secret.as_bytes())
    );
    assert!(!String::from_utf8_lossy(&json).contains(&format!("{:x}", Sha256::digest(secret))));

    let flooded = CommandSpec::catalog_read("output-bound-probe", "yes")
        .output_limit(128)
        .observed_output_limit(1024)
        .run(&context)
        .unwrap();
    let flooded_json: serde_json::Value =
        serde_json::from_slice(&flooded.to_canonical_json().unwrap()).unwrap();
    assert_eq!(flooded_json["termination"]["kind"], "output-limit");
    assert!(flooded.stdout_bytes().len() <= 128);

    let timed = CommandSpec::catalog_read("timeout-probe", "sleep")
        .public_arg(PublicArg::new("2"))
        .timeout(Duration::from_millis(20))
        .run(&context)
        .unwrap();
    let timed_json: serde_json::Value =
        serde_json::from_slice(&timed.to_canonical_json().unwrap()).unwrap();
    assert_eq!(timed_json["termination"]["kind"], "timed-out");

    let interrupted = CommandSpec::catalog_read("interrupt-probe", "sleep")
        .public_arg(PublicArg::new("2"))
        .with_interrupt_flag(requested_interrupt())
        .run(&context)
        .unwrap();
    assert!(interrupted.interrupted());
}

#[cfg(target_os = "macos")]
#[test]
fn truncated_secret_prefix_is_absent_from_canonical_run_json() {
    let fixture = RepoFixture::new("truncated-secret-prefix");
    let secret = "zzq7V5-canonical-secret";
    let cut = 7;
    let context = LiveContext::build(
        BuildRequest::new(fixture.root())
            .expect_repository_root(fixture.root())
            .expect_worktree_root(fixture.root())
            .bind_secret_source("CAPTURE_SECRET", "v1")
            .probe_tool("sandbox-exec")
            .probe_tool("printf"),
    )
    .unwrap();
    let run = CommandSpec::catalog_read("truncated-secret-prefix", "printf")
        .public_arg(PublicArg::new("%s"))
        .secret_arg(SecretArg::new(secret, "CAPTURE_SECRET"))
        .output_limit(cut)
        .observed_output_limit(cut)
        .run(&context)
        .unwrap();
    let json = run.to_canonical_json().unwrap();
    for bytes in [secret.as_bytes(), &secret.as_bytes()[..cut]] {
        assert!(!json.windows(bytes.len()).any(|seen| seen == bytes));
        assert!(
            !String::from_utf8_lossy(&json)
                .contains(&format!("sha256:{:x}", Sha256::digest(bytes)))
        );
    }
    let value: serde_json::Value = serde_json::from_slice(&json).unwrap();
    assert_eq!(
        value["termination"]["kind"],
        "withheld-secret-bearing-invocation"
    );
    assert_eq!(value["stdout"]["observation_limit_exceeded"], false);
    assert_eq!(run.monotonic_duration_ns(), 0);
    assert!(!run.interrupted());
    assert_eq!(
        value["stdout"]["captured_byte_length_is_lower_bound"],
        false
    );
}
