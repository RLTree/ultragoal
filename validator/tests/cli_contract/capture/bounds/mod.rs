use super::capture::{
    PublicArg, PublicArtifact, PublicEnv, SecretEnv, reset_test_descriptor_bytes_read,
    test_descriptor_bytes_read,
};
use super::fixture::{RepoFixture, read_command};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

#[cfg(target_os = "macos")]
#[path = "observation_budget.rs"]
mod observation_budget;

const MAX_ARGUMENTS: usize = 4096;
const MAX_ARTIFACTS: usize = 128;
const MAX_ENVIRONMENT_ENTRIES: usize = 1024;
const MAX_PATH_BYTES: usize = 4096;

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
