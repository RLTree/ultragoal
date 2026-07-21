use super::*;

#[test]
pub(crate) fn public_output_limit_is_inclusive_and_fail_closed() {
    assert!(public_output_allowed(MAX_PUBLIC_OUTPUT));
    assert!(!public_output_allowed(MAX_PUBLIC_OUTPUT + 1));
}

#[test]
pub(crate) fn public_router_adopts_only_live_successor_authority() {
    for args in [
        vec!["next"],
        vec!["--json", "inspect", "inventory"],
        vec!["package", "build", "--output", "out.json"],
        vec![
            "--json",
            "package",
            "verify",
            "--input",
            "target/ultragoal/package.hugpkg",
        ],
        vec!["observe", "query"],
        vec![
            "observe",
            "export",
            "--output",
            "out.json",
            "--approve-export",
        ],
        vec!["--help"],
    ] {
        let raw = args
            .iter()
            .map(|item| (*item).to_owned())
            .collect::<Vec<_>>();
        assert!(parse_public(&raw).is_ok(), "{args:?}");
    }
    let missing_output = vec!["--json", "package", "inventory"];
    let raw = missing_output
        .iter()
        .map(|item| (*item).to_owned())
        .collect::<Vec<_>>();
    let failure = parse_public(&raw).expect_err("canonical inventory requires output");
    assert!(failure.contains("harness-ultragoal.cli-error.v1"));
    for args in [
        vec!["--json", "package", "digest"],
        vec!["--json", "observe", "logs", "query"],
        vec!["--json", "observe", "explain-failure"],
        vec!["--json", "help"],
        vec!["--json", "current-state"],
    ] {
        let raw = args
            .iter()
            .map(|item| (*item).to_owned())
            .collect::<Vec<_>>();
        assert!(matches!(
            parse_public(&raw).expect("legacy intent yields bounded guidance"),
            ParseOutcome::Compatibility { .. }
        ));
    }
}

#[test]
pub(crate) fn fit_read_routes_are_public_and_zero_write() {
    for (action, schema) in [
        ("inspect", "RepositoryFitInspect-v1"),
        ("plan", "RepositoryFitPlan-v1"),
        ("verify", "RepositoryFitVerification-v1"),
    ] {
        let repo = Repository::new(&format!("fit-{action}"));
        let before_tree = tree(&repo.root);
        let before_status = repo.status();
        let ParseOutcome::Invocation(invocation) = parse_args(["--json", "fit", action]).unwrap()
        else {
            panic!("expected fit invocation")
        };
        let streams = execute_invocation(&repo.root, invocation).render(OutputMode::Json);
        assert!(matches!(streams.exit_code, 0 | 1), "{action}");
        assert!(streams.stderr.is_empty(), "{action}");
        let value: serde_json::Value = serde_json::from_slice(&streams.stdout).unwrap();
        assert_eq!(value["schema_version"], schema, "{action}");
        assert_eq!(tree(&repo.root), before_tree, "{action}");
        assert_eq!(repo.status(), before_status, "{action}");
    }
}

#[test]
pub(crate) fn fit_target_option_selects_the_context_root_instead_of_being_ignored() {
    let ParseOutcome::Invocation(invocation) =
        parse_args(["--json", "fit", "inspect", "--target", "nested"]).unwrap()
    else {
        panic!("expected fit invocation")
    };
    let base = Path::new("/tmp/hul-fit-public-target-root");
    assert_eq!(
        super::super::fit::target_root(base, &invocation).unwrap(),
        base.join("nested")
    );
}

#[test]
pub(crate) fn fit_apply_fails_closed_without_existing_host_state_root() {
    let repo = Repository::new("fit-apply-authority-unavailable");
    let ParseOutcome::Invocation(plan_invocation) = parse_args(["--json", "fit", "plan"]).unwrap()
    else {
        panic!("expected fit plan invocation")
    };
    let plan = execute_invocation(&repo.root, plan_invocation).render(OutputMode::Json);
    assert_eq!(plan.exit_code, 0);
    let plan_value: serde_json::Value = serde_json::from_slice(&plan.stdout).unwrap();
    let plan_sha256 = plan_value["plan"]["plan_sha256"]
        .as_str()
        .unwrap()
        .to_owned();
    let plan_path = repo.root.with_extension("fit-plan.json");
    fs::write(&plan_path, &plan.stdout).unwrap();
    let before_tree = tree(&repo.root);
    let before_status = repo.status();
    let ParseOutcome::Invocation(invocation) = parse_args([
        "--json",
        "fit",
        "apply",
        "--plan",
        plan_path.to_str().unwrap(),
        "--accept-plan",
        &plan_sha256,
    ])
    .unwrap() else {
        panic!("expected fit apply invocation")
    };
    let missing_home = repo.root.with_extension("missing-home-authority");
    let streams = execute_invocation_with_home(&repo.root, invocation, Some(&missing_home))
        .render(OutputMode::Json);
    assert!(matches!(streams.exit_code, 3 | 4));
    assert!(streams.stdout.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&streams.stderr).unwrap();
    assert!(matches!(
        value["diagnostic_id"].as_str(),
        Some("successor_runtime_authority_required")
            | Some("successor_runtime_downstream_tool_unavailable")
    ));
    assert_eq!(tree(&repo.root), before_tree);
    assert_eq!(repo.status(), before_status);
    fs::remove_file(plan_path).unwrap();
}
