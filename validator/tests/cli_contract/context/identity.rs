use super::context::{BuildRequest, ContextError, LiveContext};
use super::context_scenario::{TestDir, build, serial};
use std::fs;

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
