use super::tests::{Repo, run_git};
use super::{BuildRequest, ContextError, LiveContext, inception_subject_identity};
use crate::context::git::query;
use std::fs;

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

fn subject(repo: &Repo) -> super::inception_subject::SubjectIdentity {
    let context = LiveContext::build(BuildRequest::new(&repo.root)).unwrap();
    inception_subject_identity(&context.begin_read_session().unwrap()).unwrap()
}
