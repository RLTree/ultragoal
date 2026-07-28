use super::tests::{Repo, run_git};
use super::{BuildRequest, ContextError, LiveContext, inception_subject_identity};
use crate::context::git::{query, untracked_file::open_regular_for_test};
use std::fs::{self, OpenOptions};
use std::path::Path;

#[test]
fn recorded_git_capability_executes_fixed_query_arguments() {
    let repo = Repo::new("recorded-git-query");
    let context = LiveContext::build(BuildRequest::new(&repo.root)).unwrap();
    let session = context.begin_read_session().unwrap();
    let output = query(&session, &["rev-parse", "--show-toplevel"]).unwrap();
    assert_eq!(
        String::from_utf8(output).unwrap().trim(),
        repo.root.display().to_string()
    );
}

#[test]
fn stale_session_is_refused_before_git_query_runs() {
    let repo = Repo::new("stale-git-query");
    let context = LiveContext::build(BuildRequest::new(&repo.root)).unwrap();
    let session = context.begin_read_session().unwrap();
    fs::write(repo.root.join("tracked.txt"), b"changed\n").unwrap();
    assert!(matches!(
        query(&session, &["rev-parse", "HEAD"]),
        Err(ContextError::ConcurrentMutation(_))
    ));
}

#[test]
fn inception_subject_keeps_only_its_brief_out_of_candidate_identity() {
    let repo = Repo::new("inception-subject");
    let first = subject(&repo);
    assert!(!first.dirty);
    fs::write(repo.root.join("PRODUCT_SUCCESS_BRIEF.json"), b"first\n").unwrap();
    assert_eq!(subject(&repo), first);
    run_git(&repo.root, &["add", "PRODUCT_SUCCESS_BRIEF.json"]);
    run_git(&repo.root, &["commit", "-qm", "add brief"]);
    assert_eq!(subject(&repo), first);
    fs::write(repo.root.join("tracked.txt"), b"changed\n").unwrap();
    let changed = subject(&repo);
    assert!(changed.dirty);
    assert_ne!(changed.digest, first.digest);
}

#[cfg(unix)]
#[test]
fn inception_subject_binds_untracked_file_mode() {
    use std::os::unix::fs::PermissionsExt;

    let repo = Repo::new("inception-untracked-mode");
    let path = repo.root.join("input.sh");
    fs::write(&path, b"input\n").unwrap();
    let first = subject(&repo);
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(permissions.mode() | 0o100);
    fs::set_permissions(&path, permissions).unwrap();
    assert_ne!(subject(&repo).digest, first.digest);
}

#[test]
fn inception_subject_binds_untracked_regular_file_content() {
    let repo = Repo::new("inception-untracked-content");
    let path = repo.root.join("input.txt");
    fs::write(&path, b"first\n").unwrap();
    let first = subject(&repo);
    fs::write(&path, b"second\n").unwrap();
    assert_ne!(subject(&repo).digest, first.digest);
}

#[cfg(unix)]
#[test]
fn untracked_open_rejects_a_leaf_replaced_by_an_outside_symlink() {
    use std::os::unix::fs::symlink;

    let repo = Repo::new("untracked-leaf-swap");
    let path = repo.root.join("input.txt");
    fs::write(&path, b"inside\n").unwrap();
    let before = fs::symlink_metadata(&path).unwrap();
    let outside = repo.root.with_file_name(format!(
        "{}-outside.txt",
        repo.root.file_name().unwrap().to_string_lossy()
    ));
    fs::write(&outside, b"outside-canary\n").unwrap();
    fs::remove_file(&path).unwrap();
    symlink(&outside, &path).unwrap();

    assert!(matches!(
        open_regular_for_test(&repo.root, Path::new("input.txt"), &before),
        Err(ContextError::Io { .. } | ContextError::ConcurrentMutation(_))
    ));
    assert_eq!(fs::read(&outside).unwrap(), b"outside-canary\n");
    fs::remove_file(outside).unwrap();
}

#[cfg(unix)]
#[test]
fn untracked_open_rejects_an_ancestor_replaced_by_an_outside_symlink() {
    use std::os::unix::fs::symlink;

    let repo = Repo::new("untracked-ancestor-swap");
    let inside = repo.root.join("safe");
    fs::create_dir(&inside).unwrap();
    let path = inside.join("input.txt");
    fs::write(&path, b"inside\n").unwrap();
    let before = fs::symlink_metadata(&path).unwrap();
    let outside = repo.root.with_file_name(format!(
        "{}-outside",
        repo.root.file_name().unwrap().to_string_lossy()
    ));
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("input.txt"), b"outside-canary\n").unwrap();
    fs::rename(&inside, repo.root.join("safe-original")).unwrap();
    symlink(&outside, &inside).unwrap();

    assert!(matches!(
        open_regular_for_test(&repo.root, Path::new("safe/input.txt"), &before),
        Err(ContextError::Io { .. } | ContextError::ConcurrentMutation(_))
    ));
    assert_eq!(
        fs::read(outside.join("input.txt")).unwrap(),
        b"outside-canary\n"
    );
    fs::remove_dir_all(outside).unwrap();
}

#[test]
fn candidate_capture_rejects_oversized_untracked_regular_files() {
    let repo = Repo::new("untracked-oversized");
    let path = repo.root.join("oversized.bin");
    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .unwrap();
    file.set_len(128 * 1024 * 1024 + 1).unwrap();
    drop(file);

    assert!(matches!(
        LiveContext::build(BuildRequest::new(&repo.root)),
        Err(ContextError::PathDenied(_))
    ));
}

fn subject(repo: &Repo) -> super::inception_subject::SubjectIdentity {
    let context = LiveContext::build(BuildRequest::new(&repo.root)).unwrap();
    inception_subject_identity(&context.begin_read_session().unwrap()).unwrap()
}
