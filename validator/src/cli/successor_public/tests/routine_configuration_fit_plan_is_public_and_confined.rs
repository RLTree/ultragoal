use super::*;

#[test]
pub(crate) fn routine_configuration_plan_is_public_zero_write_and_cannot_cross_fit_routes() {
    let repo = Repository::new("routine-configuration-plan");
    fs::write(repo.root.join("AGENTS.md"), b"user-owned conflict\n").unwrap();
    fs::write(repo.root.join("operator-state.txt"), b"preserve exactly\n").unwrap();
    let before_tree = tree(&repo.root);
    let before_status = repo.status();
    let ParseOutcome::Invocation(invocation) =
        parse_args(["--json", "fit", "plan", "--routine-config"]).unwrap()
    else {
        panic!("expected routine configuration plan invocation")
    };
    let streams = execute_invocation(&repo.root, invocation).render(OutputMode::Json);
    assert_eq!(streams.exit_code, 0);
    assert!(streams.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&streams.stdout).unwrap();
    assert_eq!(value["schema_version"], "RepositoryFitPlan-v1");
    assert_eq!(value["scope"], "routine_configuration");
    assert_eq!(value["desired"]["file_count"], 2);
    assert!(value["local_state"].is_null());
    assert!(value["inspection"]["local_state"].is_null());
    assert!(value["plan"]["local_state"].is_null());
    assert_eq!(value["plan"]["conflicts"], serde_json::json!([]));
    assert_eq!(
        value["plan"]["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["path"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["config/routine-public.json", "config/routines.json"]
    );
    assert_eq!(tree(&repo.root), before_tree);
    assert_eq!(repo.status(), before_status);

    for arguments in [
        vec!["--json", "fit", "inspect", "--routine-config"],
        vec!["--json", "fit", "verify", "--routine-config"],
        vec![
            "--json",
            "fit",
            "apply",
            "--routine-config",
            "--plan",
            "/tmp/plan.json",
            "--accept-plan",
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ],
    ] {
        assert!(parse_args(arguments.clone()).is_err(), "{arguments:?}");
    }
}
