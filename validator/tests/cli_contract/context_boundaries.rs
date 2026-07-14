use super::context::{BuildRequest, ContextError, EffectClass, LiveContext};
use super::context_scenario::{TestDir, build, serial};
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

#[cfg(unix)]
#[test]
fn path_search_changes_capability_and_context_identity() {
    let _guard = serial();
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
    let original = std::env::var_os("PATH");
    let repo = TestDir::repo("path");
    let first_bin = repo.root.join("bin-one");
    let second_bin = repo.root.join("bin-two");
    fs::create_dir_all(&first_bin).unwrap();
    fs::create_dir_all(&second_bin).unwrap();
    let marker = repo.root.join("malicious-tool-ran");
    for directory in [&first_bin, &second_bin] {
        fs::write(
            directory.join("danger"),
            format!("#!/bin/sh\nprintf ran > '{}'\n", marker.display()),
        )
        .unwrap();
        fs::write(
            directory.join("git"),
            format!("#!/bin/sh\nprintf ran > '{}'\nexit 99\n", marker.display()),
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(directory.join("danger"), fs::Permissions::from_mode(0o755)).unwrap();
        fs::set_permissions(directory.join("git"), fs::Permissions::from_mode(0o755)).unwrap();
    }
    let _restore = RestorePath(original);
    unsafe { std::env::set_var("PATH", &first_bin) };
    let first = LiveContext::build(BuildRequest::new(&repo.root).probe_tool("danger")).unwrap();
    assert!(!marker.exists());
    assert_ne!(
        first
            .capabilities()
            .tool("git")
            .unwrap()
            .executable
            .as_deref(),
        Some(
            first_bin
                .join("git")
                .canonicalize()
                .unwrap()
                .to_str()
                .unwrap()
        )
    );
    assert!(first.capabilities().tool("danger").unwrap().available);
    unsafe { std::env::set_var("PATH", &second_bin) };
    let second = LiveContext::build(BuildRequest::new(&repo.root).probe_tool("danger")).unwrap();
    assert!(!marker.exists());
    assert_eq!(first.candidate(), second.candidate());
    assert_ne!(
        first.capabilities().path_search_sha256,
        second.capabilities().path_search_sha256
    );
    assert_ne!(first.context_id(), second.context_id());
}

#[test]
fn effect_and_path_boundary_deny_escalation_and_escape() {
    let _guard = serial();
    let repo = TestDir::repo("effects");
    let read = build(&repo.root);
    assert_eq!(read.effect().selected, EffectClass::Read);
    assert!(matches!(
        LiveContext::build(BuildRequest::new(&repo.root).with_effect(EffectClass::WorkspaceWrite)),
        Err(ContextError::EffectDenied(_))
    ));
}

#[test]
fn detects_concurrent_candidate_mutation() {
    let _guard = serial();
    let repo = TestDir::repo("concurrency");
    let path = repo.root.join("tracked.txt");
    let stop = Arc::new(AtomicBool::new(false));
    let writer_stop = Arc::clone(&stop);
    let writer = std::thread::spawn(move || {
        while !writer_stop.load(Ordering::Relaxed) {
            let mut file = OpenOptions::new().append(true).open(&path).unwrap();
            file.write_all(b"x").unwrap();
            std::thread::sleep(Duration::from_millis(5));
        }
    });
    let result = LiveContext::build(BuildRequest::new(&repo.root));
    stop.store(true, Ordering::Relaxed);
    writer.join().unwrap();
    assert!(matches!(result, Err(ContextError::ConcurrentMutation(_))));
}

#[test]
fn full_context_revalidation_detects_later_mutation() {
    let _guard = serial();
    let repo = TestDir::repo("revalidate");
    let context = build(&repo.root);
    fs::write(repo.root.join("tracked.txt"), b"changed\n").unwrap();
    assert!(matches!(
        context.revalidate(),
        Err(ContextError::ConcurrentMutation(_))
    ));
}

#[test]
fn linked_worktree_has_distinct_worktree_identity() {
    let _guard = serial();
    let repo = TestDir::repo("linked-main");
    let holder = TestDir::empty("linked-holder");
    let linked = holder.root.join("worktree");
    let status = Command::new("git")
        .args(["worktree", "add", "-q", "-b", "linked-context-test"])
        .arg(&linked)
        .current_dir(&repo.root)
        .status()
        .unwrap();
    assert!(status.success());
    let main = build(&repo.root);
    let alternate = build(&linked);
    assert_eq!(
        main.roots().repository_root,
        alternate.roots().repository_root
    );
    assert_ne!(main.roots().worktree_root, alternate.roots().worktree_root);
    assert_ne!(main.context_id(), alternate.context_id());
}
