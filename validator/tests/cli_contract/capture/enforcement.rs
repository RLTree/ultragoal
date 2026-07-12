use super::capture::{CommandSpec, PublicArg, PublicArtifact};
use super::fixture::{RepoFixture, read_command};
use crate::context::{BuildRequest, EffectClass, LiveContext};
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::fs;
use std::sync::Arc;
use std::sync::MutexGuard;
use std::sync::atomic::AtomicBool;

#[cfg(unix)]
use std::os::unix::fs::{PermissionsExt, symlink};

#[test]
fn outside_root_read_and_exfiltration_canary_never_reaches_the_wrapper() {
    let fixture = RepoFixture::new("outside-read");
    let outside = fixture.root().with_extension("outside-secret");
    let canary = "outside-read-canary-781";
    fs::write(&outside, canary).unwrap();
    fixture.write_script(
        "bin/read-outside",
        "IFS= read -r value < \"$1\"; printf '%s' \"$value\"",
    );
    let error = read_command("bin/read-outside")
        .public_arg(PublicArg::new(outside.as_os_str()))
        .run(&fixture.context())
        .unwrap_err();
    assert!(error.contains("accepted typed-catalog binding"));
    assert!(!error.contains(canary));
    assert!(!error.contains(&format!("sha256:{:x}", Sha256::digest(canary))));
    assert_eq!(fs::read_to_string(&outside).unwrap(), canary);
    fs::remove_file(outside).unwrap();
}

#[cfg(unix)]
#[test]
fn prohibited_worktree_environment_is_never_opened_or_walked() {
    for (index, protected) in [
        ".codex-worktree",
        ".CODEX-WORKTREE",
        ".Codex-Worktree",
        "nested/.cOdEx-WoRkTrEe",
    ]
    .into_iter()
    .enumerate()
    {
        let fixture = RepoFixture::new(&format!("prohibited-env-{index}"));
        fixture.write_file(".gitignore", format!("{protected}/\n").as_bytes());
        fixture.write_script("bin/noop", "exit 0");
        let directory = fixture.root().join(protected);
        fs::create_dir_all(&directory).unwrap();
        symlink("env.sh", directory.join("env.sh")).unwrap();
        let context = fixture.context();
        let protected_file = format!("{protected}/env.sh");

        for error in [
            read_command(&protected_file).run(&context).unwrap_err(),
            read_command("bin/noop")
                .cwd(protected)
                .run(&context)
                .unwrap_err(),
            read_command("bin/noop")
                .public_artifact(PublicArtifact::new(&protected_file))
                .run(&context)
                .unwrap_err(),
        ] {
            assert!(error.contains("prohibited secret-bearing"));
        }

        fs::set_permissions(&directory, fs::Permissions::from_mode(0o000)).unwrap();
        let observation = read_command("bin/noop")
            .with_interrupt_flag(Arc::new(AtomicBool::new(true)))
            .run(&context);
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
        assert!(
            observation
                .unwrap_err()
                .contains("accepted typed-catalog binding")
        );
    }
}

#[test]
fn wrapper_exec_and_source_canaries_are_rejected_before_launch() {
    let fixture = RepoFixture::new("wrapper-canaries");
    fixture.write_script("bin/secondary", "printf exec-ran > exec-marker");
    fixture.write_script("lib/source.sh", "printf source-ran > source-marker");
    fixture.write_script("bin/exec-wrapper", "exec /bin/sh bin/secondary");
    fixture.write_script("bin/source-wrapper", ". lib/source.sh");
    let context = fixture.context();
    for program in ["bin/exec-wrapper", "bin/source-wrapper"] {
        let error = read_command(program).run(&context).unwrap_err();
        assert!(error.contains("accepted typed-catalog binding"));
    }
    assert!(!fixture.root().join("exec-marker").exists());
    assert!(!fixture.root().join("source-marker").exists());
}

#[test]
fn write_network_fork_and_signal_canaries_have_zero_effects() {
    let fixture = RepoFixture::new("effect-canaries");
    fixture.write_script("bin/write", "printf escaped > marker");
    fixture.write_script("bin/network", "exec 3<>/dev/tcp/127.0.0.1/9");
    fixture.write_script("bin/fork", "(while :; do :; done) & printf child");
    fixture.write_script("bin/signal", "kill -TERM 1");
    let context = fixture.context();
    for program in ["bin/write", "bin/network", "bin/fork", "bin/signal"] {
        assert!(
            read_command(program)
                .run(&context)
                .unwrap_err()
                .contains("accepted typed-catalog binding")
        );
    }
    assert!(!fixture.root().join("marker").exists());
}

#[test]
fn missing_sandbox_capability_does_not_bypass_catalog_binding() {
    let fixture = RepoFixture::new("missing-sandbox");
    fixture.write_script("bin/target", "printf ran > should-not-exist");
    let context = LiveContext::build(
        BuildRequest::new(fixture.root())
            .expect_repository_root(fixture.root())
            .expect_worktree_root(fixture.root())
            .probe_tool("sh"),
    )
    .unwrap();
    let error = read_command("bin/target")
        .with_interrupt_flag(Arc::new(AtomicBool::new(true)))
        .run(&context)
        .unwrap_err();
    assert!(error.contains("accepted typed-catalog binding"));
    assert!(!fixture.root().join("should-not-exist").exists());
}

struct RestorePath(Option<OsString>);

impl Drop for RestorePath {
    fn drop(&mut self) {
        if let Some(path) = self.0.take() {
            unsafe { std::env::set_var("PATH", path) };
        } else {
            unsafe { std::env::remove_var("PATH") };
        }
    }
}

fn serial_path() -> MutexGuard<'static, ()> {
    crate::serial()
}

#[test]
fn path_shadowed_sandbox_capability_is_not_consulted_without_catalog_binding() {
    let _serial = serial_path();
    let fixture = RepoFixture::new("tampered-sandbox");
    let marker = fixture.root().join("fake-sandbox-ran");
    fixture.write_script(
        "fake-bin/sandbox-exec",
        &format!("printf ran > '{}'", marker.display()),
    );
    fixture.write_script("bin/target", "printf target");
    let original = std::env::var_os("PATH");
    let _restore = RestorePath(original.clone());
    let mut paths = vec![fixture.root().join("fake-bin")];
    paths.extend(std::env::split_paths(
        original.as_deref().unwrap_or_default(),
    ));
    unsafe { std::env::set_var("PATH", std::env::join_paths(paths).unwrap()) };
    let context = LiveContext::build(
        BuildRequest::new(fixture.root())
            .probe_tool("sandbox-exec")
            .probe_tool("sh"),
    )
    .unwrap();
    let error = read_command("bin/target")
        .with_interrupt_flag(Arc::new(AtomicBool::new(true)))
        .run(&context)
        .unwrap_err();
    assert!(error.contains("accepted typed-catalog binding"));
    assert!(!marker.exists());
}

#[cfg(target_os = "macos")]
#[test]
fn non_shebang_program_is_also_rejected_before_launch() {
    let fixture = RepoFixture::new("non-shebang-denial");
    fixture.write_file("bin/raw", b"not a descriptor-safe executable\n");
    let path = fixture.root().join("bin/raw");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    let error = CommandSpec::new("bin/raw", EffectClass::Read)
        .run(&fixture.context())
        .unwrap_err();
    assert!(error.contains("accepted typed-catalog binding"));
}

#[cfg(unix)]
#[test]
fn linked_worktree_rejects_before_revalidation_or_fifo_descriptor_open() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    use std::process::Command;
    use std::time::{Duration, Instant};

    let fixture = RepoFixture::new("linked-worktree-root");
    fixture.write_file("tracked", b"tracked\n");
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(fixture.root())
            .args(["add", "tracked"])
            .status()
            .unwrap()
            .success()
    );
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(fixture.root())
            .args([
                "-c",
                "user.name=Capture Test",
                "-c",
                "user.email=capture@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "-qm",
                "initial",
            ])
            .status()
            .unwrap()
            .success()
    );
    let linked = fixture.root().with_extension("linked-worktree");
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(fixture.root())
            .args(["worktree", "add", "-q", "-b", "capture-linked"])
            .arg(&linked)
            .status()
            .unwrap()
            .success()
    );
    fs::create_dir(linked.join("bin")).unwrap();
    let fifo = linked.join("bin/program");
    let fifo_c = CString::new(fifo.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo_c.as_ptr(), 0o700) }, 0);
    let context = LiveContext::build(
        BuildRequest::new(&linked)
            .expect_repository_root(fixture.root())
            .expect_worktree_root(&linked)
            .probe_tool("sandbox-exec")
            .probe_tool("sh"),
    )
    .unwrap();

    fs::write(linked.join("revalidation-canary"), b"drift").unwrap();
    let started = Instant::now();
    let error = read_command("bin/program")
        .with_interrupt_flag(Arc::new(AtomicBool::new(true)))
        .run(&context)
        .unwrap_err();
    assert!(error.contains("alternate repository/worktree roots"));
    assert!(
        started.elapsed() < Duration::from_millis(500),
        "alternate-root rejection must not block on the FIFO"
    );

    assert!(
        Command::new("git")
            .arg("-C")
            .arg(fixture.root())
            .args(["worktree", "remove", "--force"])
            .arg(&linked)
            .status()
            .unwrap()
            .success()
    );
}
