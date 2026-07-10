use super::request::{BuildRequest, RootEffectGrant};
use super::{ContextError, EffectClass, LiveContext};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

pub(super) struct Repo {
    pub(super) root: PathBuf,
}

impl Repo {
    pub(super) fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ultragoal-context-unit-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        let root = root.canonicalize().unwrap();
        run_git(&root, &["init", "-q"]);
        run_git(&root, &["config", "user.email", "context@example.invalid"]);
        run_git(&root, &["config", "user.name", "Context Test"]);
        fs::write(root.join("tracked.txt"), b"inside\n").unwrap();
        run_git(&root, &["add", "tracked.txt"]);
        run_git(&root, &["commit", "-q", "-m", "fixture"]);
        Self { root }
    }

    fn write_context(&self) -> LiveContext {
        let grant = RootEffectGrant::issue(EffectClass::WorkspaceWrite, vec![self.root.clone()]);
        LiveContext::build(
            BuildRequest::new(&self.root)
                .with_effect(EffectClass::WorkspaceWrite)
                .with_root_grant(grant),
        )
        .unwrap()
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub(super) fn run_git(root: &Path, args: &[&str]) {
    assert!(
        Command::new("git")
            .args(args)
            .current_dir(root)
            .status()
            .unwrap()
            .success()
    );
}

#[test]
fn descriptor_bound_read_and_create_work_without_raw_public_paths() {
    let repo = Repo::new("descriptor");
    let read_context = LiveContext::build(BuildRequest::new(&repo.root)).unwrap();
    let read = read_context
        .authorize_path("tracked.txt", EffectClass::Read)
        .unwrap();
    let mut contents = String::new();
    read.open_read()
        .unwrap()
        .read_to_string(&mut contents)
        .unwrap();
    assert_eq!(contents, "inside\n");

    let write_context = repo.write_context();
    let target = write_context
        .authorize_path("created.txt", EffectClass::WorkspaceWrite)
        .unwrap();
    assert!(!target.existed());
    let mut file = target.create_new_write().unwrap();
    file.write_all(b"created safely\n").unwrap();
    drop(file);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(repo.root.join("created.txt"))
                .unwrap()
                .permissions()
                .mode()
                & 0o600,
            0o600
        );
    }
    assert_eq!(
        fs::read(repo.root.join("created.txt")).unwrap(),
        b"created safely\n"
    );
}

#[cfg(unix)]
#[test]
fn projected_symlink_swap_is_rejected_without_outside_write() {
    let repo = Repo::new("symlink-swap");
    let outside = std::env::temp_dir().join(format!(
        "ultragoal-context-outside-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&outside).unwrap();
    let outside_file = outside.join("target.txt");
    fs::write(&outside_file, b"untouched\n").unwrap();
    let context = repo.write_context();
    let target = context
        .authorize_path("new-dir/target.txt", EffectClass::WorkspaceWrite)
        .unwrap();
    std::os::unix::fs::symlink(&outside, repo.root.join("new-dir")).unwrap();
    assert!(matches!(
        target.create_new_write(),
        Err(ContextError::PathDenied(_))
    ));
    assert_eq!(fs::read(&outside_file).unwrap(), b"untouched\n");
    let _ = fs::remove_dir_all(outside);
}

#[cfg(unix)]
#[test]
fn hardlink_and_existing_symlink_swaps_return_no_write_descriptor() {
    let repo = Repo::new("link-swaps");
    let outside = std::env::temp_dir().join(format!(
        "ultragoal-context-links-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&outside).unwrap();
    let outside_file = outside.join("outside.txt");
    fs::write(&outside_file, b"outside stays\n").unwrap();
    let context = repo.write_context();

    let new_target = context
        .authorize_path("new.txt", EffectClass::WorkspaceWrite)
        .unwrap();
    fs::hard_link(&outside_file, repo.root.join("new.txt")).unwrap();
    assert!(new_target.create_new_write().is_err());
    fs::remove_file(repo.root.join("new.txt")).unwrap();

    let existing = repo
        .write_context()
        .authorize_path("tracked.txt", EffectClass::WorkspaceWrite)
        .unwrap();
    fs::remove_file(repo.root.join("tracked.txt")).unwrap();
    std::os::unix::fs::symlink(&outside_file, repo.root.join("tracked.txt")).unwrap();
    assert!(existing.open_existing_write().is_err());
    assert_eq!(fs::read(&outside_file).unwrap(), b"outside stays\n");
    let _ = fs::remove_dir_all(outside);
}

#[cfg(unix)]
#[test]
fn parent_swap_during_open_cannot_create_an_empty_outside_target() {
    use super::authorized_io::set_test_pause_before_open;
    let repo = Repo::new("open-race");
    let safe = repo.root.join("safe");
    fs::create_dir(&safe).unwrap();
    let outside = std::env::temp_dir().join(format!(
        "ultragoal-context-open-race-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&outside).unwrap();
    let context = repo.write_context();
    let target = context
        .authorize_path("safe/created.txt", EffectClass::WorkspaceWrite)
        .unwrap();
    let safe_for_swap = safe.clone();
    let outside_for_swap = outside.clone();
    set_test_pause_before_open(150);
    let swap = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(30));
        fs::rename(
            &safe_for_swap,
            safe_for_swap.with_file_name("safe-original"),
        )
        .unwrap();
        std::os::unix::fs::symlink(&outside_for_swap, &safe_for_swap).unwrap();
    });
    let result = target.create_new_write();
    swap.join().unwrap();
    assert!(result.is_err());
    assert!(!outside.join("created.txt").exists());
    let _ = fs::remove_dir_all(outside);
}

#[cfg(unix)]
#[test]
fn whole_worktree_replacement_cannot_reuse_path_authority() {
    let repo = Repo::new("root-replacement");
    let context = repo.write_context();
    let target = context
        .authorize_path("created.txt", EffectClass::WorkspaceWrite)
        .unwrap();
    let moved = repo.root.with_file_name(format!(
        "ultragoal-context-moved-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::rename(&repo.root, &moved).unwrap();
    fs::create_dir(&repo.root).unwrap();
    let result = target.create_new_write();
    assert!(matches!(
        result,
        Err(ContextError::PathDenied(_) | ContextError::ConcurrentMutation(_))
    ));
    assert!(!repo.root.join("created.txt").exists());
    fs::remove_dir(&repo.root).unwrap();
    fs::rename(moved, &repo.root).unwrap();
}

#[test]
fn public_effect_declaration_cannot_self_issue_authority() {
    let repo = Repo::new("grant");
    assert!(matches!(
        LiveContext::build(BuildRequest::new(&repo.root).with_effect(EffectClass::Destructive)),
        Err(ContextError::EffectDenied(_))
    ));
}
