use super::capture::{PublicArg, PublicEnv, SecretArg, SecretEnv};
use super::fixture::{RepoFixture, read_command};
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
