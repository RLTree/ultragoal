use super::*;

#[test]
fn evaluation_run_refuses_before_context_or_workspace_writes() {
    let repo = Repository::new("evaluation-run");
    let ParseOutcome::Invocation(invocation) = parse_args([
        "--json",
        "eval",
        "run",
        "--spec",
        "evaluation/run.json",
        "--output",
        "results/run.json",
    ])
    .unwrap() else {
        panic!("expected evaluation run invocation");
    };
    let before_tree = tree(&repo.root);
    let before_status = repo.status();
    let streams = execute_invocation(&repo.root, invocation).render(OutputMode::Json);
    assert_eq!(
        streams.exit_code,
        4,
        "{}",
        String::from_utf8_lossy(&streams.stderr)
    );
    assert!(streams.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&streams.stderr)
            .contains("successor_runtime_downstream_tool_unavailable")
    );
    assert!(!repo.root.join("results/run.json").exists());
    assert!(!repo.root.join(".ultragoal/evaluation").exists());
    assert!(!repo.root.join(".ultragoal-evaluation-runs").exists());
    assert_eq!(tree(&repo.root), before_tree);
    assert_eq!(repo.status(), before_status);
}
