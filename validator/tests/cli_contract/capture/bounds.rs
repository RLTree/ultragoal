use super::capture::{
    CommandSpec, PublicArg, PublicArtifact, PublicEnv, SecretEnv, reset_test_descriptor_bytes_read,
    test_descriptor_bytes_read,
};
use super::fixture::{RepoFixture, read_command};
#[cfg(target_os = "macos")]
use crate::context::{BuildRequest, LiveContext};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

const MAX_ARGUMENTS: usize = 4096;
const MAX_ARTIFACTS: usize = 128;
const MAX_ENVIRONMENT_ENTRIES: usize = 1024;
const MAX_PATH_BYTES: usize = 4096;

#[cfg(target_os = "macos")]
fn catalog_context(fixture: &RepoFixture) -> LiveContext {
    LiveContext::build(
        BuildRequest::new(fixture.root())
            .expect_repository_root(fixture.root())
            .expect_worktree_root(fixture.root())
            .probe_tool("sandbox-exec")
            .probe_tool("printf"),
    )
    .unwrap()
}

#[cfg(target_os = "macos")]
#[test]
fn finite_successful_output_cannot_race_past_the_combined_observation_limit() {
    let fixture = RepoFixture::new("finite-output-race");
    let context = catalog_context(&fixture);
    let payload = "x".repeat(4096);

    std::thread::scope(|scope| {
        for iteration in 0..24 {
            let context = &context;
            let payload = &payload;
            scope.spawn(move || {
                let run = CommandSpec::catalog_read("finite-output-race", "printf")
                    .public_arg(PublicArg::new("%s"))
                    .public_arg(PublicArg::new(payload))
                    .output_limit(64)
                    .observed_output_limit(64)
                    .run(context)
                    .unwrap();
                let value: serde_json::Value =
                    serde_json::from_slice(&run.to_canonical_json().unwrap()).unwrap();
                assert_eq!(
                    value["termination"]["kind"], "output-limit",
                    "finite iteration {iteration} must not retain the child's zero exit"
                );
                assert_eq!(value["stdout"]["truncated"], true);
                assert_eq!(value["stdout"]["observation_limit_exceeded"], true);
                assert_eq!(value["stdout"]["captured_byte_length_is_lower_bound"], true);
            });
        }
    });
}

#[cfg(target_os = "macos")]
#[test]
fn finite_output_at_the_exact_observation_boundary_is_complete_success() {
    let fixture = RepoFixture::new("finite-output-exact-boundary");
    let context = catalog_context(&fixture);
    let payload = "x".repeat(64);

    let run = CommandSpec::catalog_read("finite-output-exact-boundary", "printf")
        .public_arg(PublicArg::new("%s"))
        .public_arg(PublicArg::new(&payload))
        .output_limit(64)
        .observed_output_limit(64)
        .run(&context)
        .unwrap();
    let value: serde_json::Value =
        serde_json::from_slice(&run.to_canonical_json().unwrap()).unwrap();

    assert_eq!(value["termination"]["kind"], "exited");
    assert_eq!(value["termination"]["code"], 0);
    assert_eq!(value["stdout"]["captured_byte_length"], 64);
    assert_eq!(value["stdout"]["truncated"], false);
    assert_eq!(value["stdout"]["observation_limit_exceeded"], false);
    assert_eq!(
        value["stdout"]["captured_byte_length_is_lower_bound"],
        false
    );
}

#[cfg(target_os = "macos")]
#[test]
fn finite_stdout_and_stderr_share_one_observation_budget() {
    let fixture = RepoFixture::new("finite-combined-output");
    let context = catalog_context(&fixture);
    let format = format!("{}%", "x".repeat(48));

    std::thread::scope(|scope| {
        for iteration in 0..16 {
            let context = &context;
            let format = &format;
            scope.spawn(move || {
                let run = CommandSpec::catalog_read("finite-combined-output", "printf")
                    .public_arg(PublicArg::new(format))
                    .output_limit(64)
                    .observed_output_limit(64)
                    .run(context)
                    .unwrap();
                let value: serde_json::Value =
                    serde_json::from_slice(&run.to_canonical_json().unwrap()).unwrap();
                assert_eq!(value["termination"]["kind"], "output-limit");
                assert!(
                    value["stdout"]["truncated"] == true || value["stderr"]["truncated"] == true,
                    "iteration {iteration} must disclose the combined-budget loser"
                );
                assert!(
                    value["stdout"]["observation_limit_exceeded"] == true
                        || value["stderr"]["observation_limit_exceeded"] == true
                );
            });
        }
    });
}

#[test]
fn cardinality_bounds_reject_before_any_item_is_inspected() {
    let fixture = RepoFixture::new("cardinality-bounds");
    let context = fixture.context();

    let mut arguments = read_command("bin/missing").public_arg(PublicArg::new("contains\0nul"));
    for _ in 1..=MAX_ARGUMENTS {
        arguments = arguments.public_arg(PublicArg::new("x"));
    }
    assert_eq!(
        arguments.run(&context).unwrap_err(),
        "argv exceeds the supported bound"
    );

    let mut environment = read_command("bin/missing").secret_environment(SecretEnv::new(
        "FIRST",
        "x",
        "unbound-secret-source",
    ));
    for index in 1..=MAX_ENVIRONMENT_ENTRIES {
        environment = environment.secret_environment(SecretEnv::new(
            format!("SECRET_{index}"),
            "x",
            "unbound-secret-source",
        ));
    }
    assert_eq!(
        environment.run(&context).unwrap_err(),
        "environment allowlist exceeds entry bound"
    );

    let mut artifacts =
        read_command("bin/missing").public_artifact(PublicArtifact::new(".codex-worktree/env.sh"));
    for index in 1..=MAX_ARTIFACTS {
        artifacts = artifacts.public_artifact(PublicArtifact::new(format!("evidence/{index}")));
    }
    assert_eq!(
        artifacts.run(&context).unwrap_err(),
        "artifact expectation count exceeds the supported bound"
    );
}

#[test]
fn path_byte_bounds_precede_component_scans_and_descriptor_access() {
    let fixture = RepoFixture::new("path-byte-bounds");
    fixture.write_script("bin/marker", "printf ran > target-ran");
    let context = fixture.context();
    let canary = "path-bound-canary-913";
    let digest = format!("sha256:{:x}", Sha256::digest(canary));
    let oversized = format!(".codex-worktree/{canary}/{}", "x".repeat(MAX_PATH_BYTES));

    reset_test_descriptor_bytes_read();
    let errors = [
        read_command(&oversized).run(&context).unwrap_err(),
        read_command("bin/marker")
            .cwd(&oversized)
            .run(&context)
            .unwrap_err(),
        read_command("bin/marker")
            .public_artifact(PublicArtifact::new(&oversized))
            .run(&context)
            .unwrap_err(),
    ];
    for error in errors {
        assert!(error.contains("path byte bound"));
        assert!(!error.contains("prohibited secret-bearing"));
        assert!(!error.contains(canary));
        assert!(!error.contains(&digest));
    }
    assert_eq!(test_descriptor_bytes_read(), 0);
    assert!(!fixture.root().join("target-ran").exists());
}

#[test]
fn exact_path_byte_bounds_reach_the_fixed_catalog_boundary() {
    let fixture = RepoFixture::new("exact-path-byte-bounds");
    fixture.write_script("bin/existing", "exit 0");
    let context = fixture.context();
    let expected = read_command("bin/existing").run(&context).unwrap_err();
    assert!(expected.contains("accepted typed-catalog binding"));

    let exact = "x".repeat(MAX_PATH_BYTES);
    for actual in [
        read_command(&exact).run(&context).unwrap_err(),
        read_command("bin/existing")
            .cwd(&exact)
            .run(&context)
            .unwrap_err(),
        read_command("bin/existing")
            .public_artifact(PublicArtifact::new(&exact))
            .run(&context)
            .unwrap_err(),
    ] {
        assert_eq!(actual, expected);
    }
}

fn maximal_request(program: &str, interrupted: bool) -> super::capture::CommandSpec {
    let mut spec = read_command(program);
    for _ in 0..MAX_ARGUMENTS {
        spec = spec.public_arg(PublicArg::new("x"));
    }
    for index in 0..MAX_ENVIRONMENT_ENTRIES {
        spec = spec.public_environment(PublicEnv::new(format!("PUBLIC_{index}"), "x"));
    }
    for index in 0..MAX_ARTIFACTS {
        spec = spec.public_artifact(PublicArtifact::new(format!("evidence/{index}")));
    }
    spec.with_interrupt_flag(Arc::new(AtomicBool::new(interrupted)))
}

#[cfg(unix)]
#[test]
fn maximum_cardinalities_preserve_one_catalog_boundary_for_all_program_states() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    use std::time::{Duration, Instant};

    let fixture = RepoFixture::new("maximum-cardinalities");
    fixture.write_script("bin/existing", "exit 0");
    let fifo = fixture.root().join("bin/fifo");
    let fifo_c = CString::new(fifo.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo_c.as_ptr(), 0o700) }, 0);
    let context = fixture.context();
    let expected = maximal_request("bin/existing", false)
        .run(&context)
        .unwrap_err();
    assert!(expected.contains("accepted typed-catalog binding"));

    reset_test_descriptor_bytes_read();
    let started = Instant::now();
    for program in ["bin/existing", "bin/missing", "bin/fifo"] {
        for interrupted in [false, true] {
            assert_eq!(
                maximal_request(program, interrupted)
                    .run(&context)
                    .unwrap_err(),
                expected
            );
        }
    }
    assert!(started.elapsed() < Duration::from_secs(3));
    assert_eq!(test_descriptor_bytes_read(), 0);
}
