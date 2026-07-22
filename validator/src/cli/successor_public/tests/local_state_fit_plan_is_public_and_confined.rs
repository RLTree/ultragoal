use super::*;

#[test]
pub(crate) fn local_state_plan_is_public_zero_write_and_cannot_cross_fit_routes() {
    let repo = Repository::new("local-state-plan");
    fs::write(repo.root.join("AGENTS.md"), b"user-owned conflict\n").unwrap();
    fs::write(repo.root.join(".gitignore"), b"/target/\n").unwrap();
    let before_tree = tree(&repo.root);
    let before_status = repo.status();
    let ParseOutcome::Invocation(invocation) =
        parse_args(["--json", "fit", "plan", "--local-state"]).unwrap()
    else {
        panic!("expected local state plan invocation")
    };
    let streams = execute_invocation(&repo.root, invocation).render(OutputMode::Json);
    assert_eq!(streams.exit_code, 0);
    let value: serde_json::Value = serde_json::from_slice(&streams.stdout).unwrap();
    assert_eq!(value["scope"], "local_state");
    assert_eq!(value["desired"]["file_count"], 0);
    assert_eq!(value["plan"]["mutations"], serde_json::json!([]));
    assert_eq!(value["local_state"]["path"], ".gitignore");
    assert_eq!(tree(&repo.root), before_tree);
    assert_eq!(repo.status(), before_status);

    for arguments in [
        vec!["--json", "fit", "inspect", "--local-state"],
        vec!["--json", "fit", "verify", "--local-state"],
    ] {
        assert!(parse_args(arguments.clone()).is_err(), "{arguments:?}");
    }
    let ParseOutcome::Invocation(invocation) =
        parse_args(["--json", "fit", "plan", "--routine-config", "--local-state"]).unwrap()
    else {
        panic!("expected mixed scope invocation")
    };
    let rejected = execute_invocation(&repo.root, invocation).render(OutputMode::Json);
    assert_ne!(rejected.exit_code, 0);
    assert_eq!(tree(&repo.root), before_tree);
    assert_eq!(repo.status(), before_status);
}
