use super::tests::Repo;
use super::{BuildRequest, ContextError, LiveContext};
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
