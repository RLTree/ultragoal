use super::capture::{CommandSpec, PublicArg, PublicEnv, SecretArg, SecretEnv};
use super::fixture::{RepoFixture, read_command};
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

#[test]
fn shell_execution_is_rejected_before_the_target_runs() {
    let fixture = RepoFixture::new("execution-disabled");
    fixture.write_script("bin/target", "printf ran > target-ran");
    let error = read_command("bin/target")
        .run(&fixture.context())
        .unwrap_err();
    assert!(error.contains("accepted typed-catalog binding"));
    assert!(!fixture.root().join("target-ran").exists());
}

#[test]
fn pre_requested_interruption_cannot_bypass_catalog_binding() {
    let fixture = RepoFixture::new("pre-interruption");
    fixture.write_script(
        "bin/should-not-run",
        "printf unexpected > unexpected; exit 9",
    );
    let error = read_command("bin/should-not-run")
        .with_interrupt_flag(requested_interrupt())
        .run(&fixture.context())
        .unwrap_err();
    assert!(error.contains("accepted typed-catalog binding"));
    assert!(!fixture.root().join("unexpected").exists());
}

#[test]
fn timeout_environment_and_observed_output_budgets_fail_before_spawn() {
    let fixture = RepoFixture::new("request-bounds");
    fixture.write_script("bin/noop", "exit 0");
    let context = fixture.context_with_secret("BOUND", "rotation-v1");
    assert!(
        read_command("bin/noop")
            .timeout(Duration::MAX)
            .run(&context)
            .unwrap_err()
            .contains("timeout")
    );
    assert!(
        read_command("bin/noop")
            .secret_environment(SecretEnv::new("TOKEN", "x".repeat(5000), "BOUND"))
            .run(&context)
            .unwrap_err()
            .contains("accepted typed-catalog binding")
    );
    assert!(
        read_command("bin/noop")
            .output_limit(4096)
            .observed_output_limit(1024)
            .run(&context)
            .unwrap_err()
            .contains("observed output limit")
    );
}

#[test]
fn catalog_rejection_never_echoes_or_digests_secret_inputs() {
    let fixture = RepoFixture::new("secret-records");
    fixture.write_script("bin/noop", "exit 0");
    let context = fixture.context_with_secret("BOUND", "rotation-v9");
    let canary = "low-entropy-needle-77";
    let digest = format!("sha256:{:x}", Sha256::digest(canary));
    let error = read_command("bin/noop")
        .secret_arg(SecretArg::new(canary, "BOUND"))
        .secret_environment(SecretEnv::new("TOKEN", canary, "BOUND"))
        .with_interrupt_flag(requested_interrupt())
        .run(&context)
        .unwrap_err();
    assert!(error.contains("accepted typed-catalog binding"));
    assert!(!error.contains(canary));
    assert!(!error.contains(&digest));
}

#[test]
fn caller_classified_public_argument_and_environment_fail_closed() {
    let fixture = RepoFixture::new("public-records");
    fixture.write_script("bin/noop", "exit 0");
    let argument_error = read_command("bin/noop")
        .public_arg(PublicArg::new("visible-arg"))
        .run(&fixture.context())
        .unwrap_err();
    let environment_error = read_command("bin/noop")
        .public_environment(PublicEnv::new("VISIBLE", "yes"))
        .run(&fixture.context())
        .unwrap_err();
    let sensitive_name_error = read_command("bin/noop")
        .public_environment(PublicEnv::new("TOKEN", "caller-label"))
        .run(&fixture.context())
        .unwrap_err();
    assert!(argument_error.contains("accepted typed-catalog binding"));
    assert!(environment_error.contains("accepted typed-catalog binding"));
    assert!(sensitive_name_error.contains("accepted typed-catalog binding"));
}

#[test]
fn public_non_utf8_argument_is_rejected_without_inspection_or_echo() {
    let fixture = RepoFixture::new("non-utf8");
    fixture.write_script("bin/noop", "exit 0");
    #[cfg(unix)]
    let argument = OsString::from_vec(vec![0xff, 0xfe, b'A']);
    #[cfg(not(unix))]
    let argument = OsString::from("fallback");
    let error = read_command("bin/noop")
        .public_arg(PublicArg::new(argument))
        .run(&fixture.context())
        .unwrap_err();
    assert!(error.contains("accepted typed-catalog binding"));
}

#[cfg(unix)]
#[test]
fn existing_missing_and_fifo_programs_share_one_bounded_catalog_rejection() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    use std::time::Instant;

    let fixture = RepoFixture::new("program-oracle");
    fixture.write_script("bin/existing", "exit 0");
    let fifo = fixture.root().join("bin/fifo");
    let fifo_c = CString::new(fifo.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo_c.as_ptr(), 0o700) }, 0);
    let context = fixture.context();
    let expected = read_command("bin/existing").run(&context).unwrap_err();
    assert!(expected.contains("accepted typed-catalog binding"));
    let started = Instant::now();
    for path in ["bin/missing", "bin/fifo", "bin/existing"] {
        for interrupted in [false, true] {
            let flag = Arc::new(AtomicBool::new(interrupted));
            let actual = read_command(path)
                .with_interrupt_flag(flag)
                .run(&context)
                .unwrap_err();
            assert_eq!(
                actual, expected,
                "program path leaked an availability oracle"
            );
        }
    }
    assert!(
        started.elapsed() < Duration::from_millis(500),
        "catalog rejection must not block on the FIFO"
    );
}
