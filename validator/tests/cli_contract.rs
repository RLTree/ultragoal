#[cfg(unix)]
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
use ultragoal::context::{BuildRequest, ContextError, EffectClass, LiveContext};

mod context {
    pub use ultragoal::context::*;
}

#[path = "cli_contract/capture/mod.rs"]
mod successor_capture;

#[path = "cli_contract/state/mod.rs"]
mod state_contract;

#[path = "cli_contract/successor/mod.rs"]
mod successor_cli_contract;

static NEXT_DIR: AtomicU64 = AtomicU64::new(0);
static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|error| error.into_inner())
}

struct TestDir {
    root: PathBuf,
}

impl TestDir {
    fn empty(label: &str) -> Self {
        let sequence = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "ultragoal-context-{label}-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn repo(label: &str) -> Self {
        let fixture = Self::empty(label);
        fixture.git(&["init", "-q"]);
        fixture.git(&["config", "user.email", "context@example.invalid"]);
        fixture.git(&["config", "user.name", "Context Test"]);
        fs::write(fixture.root.join("tracked.txt"), b"one\n").unwrap();
        fixture.git(&["add", "tracked.txt"]);
        fixture.git(&["commit", "-q", "-m", "fixture"]);
        fixture
    }

    fn git(&self, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(&self.root)
            .env("GIT_OPTIONAL_LOCKS", "0")
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?}");
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn build(root: &Path) -> LiveContext {
    LiveContext::build(BuildRequest::new(root)).unwrap()
}

#[test]
fn repeatable_across_nested_and_symlink_aliases() {
    let _guard = serial();
    let repo = TestDir::repo("aliases");
    fs::create_dir(repo.root.join("nested")).unwrap();
    let direct = build(&repo.root);
    let nested = build(&repo.root.join("nested"));
    assert_eq!(direct.context_id(), nested.context_id());
    #[cfg(unix)]
    {
        let aliases = TestDir::empty("symlink");
        let alias = aliases.root.join("repo-link");
        std::os::unix::fs::symlink(&repo.root, &alias).unwrap();
        assert_eq!(direct.context_id(), build(&alias).context_id());
    }
}

#[test]
fn candidate_binds_dirty_and_untracked_content() {
    let _guard = serial();
    let repo = TestDir::repo("dirty");
    fs::write(repo.root.join("untracked.txt"), b"first").unwrap();
    let first = build(&repo.root);
    fs::write(repo.root.join("untracked.txt"), b"second").unwrap();
    let second = build(&repo.root);
    assert!(first.candidate().dirty && second.candidate().dirty);
    assert_eq!(
        first.candidate().status_sha256,
        second.candidate().status_sha256
    );
    assert_ne!(
        first.candidate().untracked_content_sha256,
        second.candidate().untracked_content_sha256
    );
    assert_ne!(first.context_id(), second.context_id());
}

#[test]
fn rejects_outside_git_and_ambiguous_expected_root() {
    let _guard = serial();
    let outside = TestDir::empty("outside");
    assert!(matches!(
        LiveContext::build(BuildRequest::new(&outside.root)),
        Err(ContextError::NotGitRepository(_))
    ));
    let first = TestDir::repo("root-one");
    let second = TestDir::repo("root-two");
    assert!(matches!(
        LiveContext::build(BuildRequest::new(&first.root).expect_worktree_root(&second.root)),
        Err(ContextError::RootMismatch { .. })
    ));
}

#[test]
fn selected_inputs_configuration_and_missing_tools_are_bound_without_value_disclosure() {
    let _guard = serial();
    let repo = TestDir::repo("inputs");
    let context = LiveContext::build(
        BuildRequest::new(&repo.root)
            .select_input("tracked.txt")
            .bind_non_secret_configuration("profile", "routine")
            .bind_secret_source("openai", "env-v1")
            .probe_tool("definitely-missing-ultragoal-tool"),
    )
    .unwrap();
    assert_eq!(context.selected_inputs()[0].relative_path, "tracked.txt");
    assert!(
        !context
            .capabilities()
            .tool("definitely-missing-ultragoal-tool")
            .unwrap()
            .available
    );
    let json = String::from_utf8(context.to_canonical_json().unwrap()).unwrap();
    assert!(json.contains("routine"));
    assert!(json.contains("env-v1"));
    assert!(matches!(
        LiveContext::build(
            BuildRequest::new(&repo.root)
                .bind_non_secret_configuration("api_key", "sk-do-not-bind")
        ),
        Err(ContextError::InvalidRequest(_))
    ));
}

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
