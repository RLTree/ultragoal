#![cfg(unix)]

use super::capture::CommandSpec;
use super::fixture::RepoFixture;
use crate::context::{BuildRequest, LiveContext};
use std::ffi::CString;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

fn read_command(program: &str) -> CommandSpec {
    super::fixture::read_command(program)
}

#[test]
fn linked_worktree_rejects_before_revalidation_or_fifo_descriptor_open() {
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
