use super::*;

#[test]
pub(crate) fn unavailable_context_is_stable_and_does_not_echo_input() {
    let canary = "/missing/successor-public-canary-3921";
    let ParseOutcome::Invocation(invocation) =
        parse_args(["--json", "inspect", "context"]).unwrap()
    else {
        panic!("expected invocation")
    };
    let streams = execute_invocation(Path::new(canary), invocation).render(OutputMode::Json);
    assert_eq!(streams.exit_code, 4);
    assert!(streams.stdout.is_empty());
    let text = String::from_utf8(streams.stderr).unwrap();
    assert!(text.contains("successor_runtime_context_unavailable"));
    assert!(!text.contains("successor-public-canary-3921"));
}

#[test]
pub(crate) fn public_context_uses_opaque_root_ids_and_is_recursively_zero_write() {
    let repo = Repository::new("public-context-roots");
    fs::write(repo.root.join("dirty-canary.txt"), b"dirty\n").unwrap();
    let before_tree = tree(&repo.root);
    let before_status = repo.status();
    let ParseOutcome::Invocation(invocation) =
        parse_args(["--json", "inspect", "context"]).unwrap()
    else {
        panic!("expected invocation")
    };

    let streams = execute_invocation(&repo.root, invocation).render(OutputMode::Json);
    assert_eq!(streams.exit_code, 0);
    assert!(streams.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&streams.stdout).unwrap();
    assert_eq!(value["schema_version"], "HarnessPublicContext-v2");
    assert_eq!(value["candidate"]["dirty"], true);
    for key in ["repository_root_id", "worktree_root_id"] {
        assert!(
            value["roots"][key]
                .as_str()
                .is_some_and(|id| id.starts_with("sha256:") && id.len() == 71)
        );
    }
    assert!(value["roots"].get("repository_root").is_none());
    assert!(value["roots"].get("worktree_root").is_none());
    assert_eq!(value["runtime"]["self_bound"], false);
    assert_eq!(value["runtime"]["version"], env!("CARGO_PKG_VERSION"));
    assert!(
        value["runtime"]["executable_sha256"]
            .as_str()
            .is_some_and(|digest| digest.starts_with("sha256:") && digest.len() == 71)
    );
    assert!(
        value["runtime"]["executable_byte_length"]
            .as_u64()
            .is_some_and(|length| length > 0)
    );
    assert!(value["runtime"].get("executable_path").is_none());
    assert!(!String::from_utf8_lossy(&streams.stdout).contains(repo.root.to_str().unwrap()));
    assert_eq!(tree(&repo.root), before_tree);
    assert_eq!(repo.status(), before_status);
}

#[test]
pub(crate) fn repository_fit_required_is_typed_read_only_guidance() {
    let repo = Repository::new("repository-fit-required");
    let context = read_context(&repo.root).unwrap();
    let before_tree = tree(&repo.root);
    let before_status = repo.status();
    let ParseOutcome::Invocation(invocation) = parse_args(["--json", "next"]).unwrap() else {
        panic!("expected invocation")
    };

    let streams =
        super::super::fit::repository_fit_required(&context, &invocation).render(OutputMode::Json);
    assert_eq!(streams.exit_code, 1);
    assert!(streams.stdout.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&streams.stderr).unwrap();
    assert_eq!(value["diagnostic_id"], "repository_fit_required");
    assert_eq!(value["effect"], "read");
    assert_eq!(
        value["exact_rerun"],
        "ultragoal --json fit inspect --target ."
    );
    assert!(
        value["resulting_ceiling"]
            .as_str()
            .is_some_and(|ceiling| ceiling.contains("claim_effect=none"))
    );
    assert_eq!(tree(&repo.root), before_tree);
    assert_eq!(repo.status(), before_status);
}

#[test]
pub(crate) fn public_context_output_failure_never_falls_back_to_private_live_context() {
    let repo = Repository::new("public-context-output-failure");
    let context = read_context(&repo.root).unwrap();
    let before_tree = tree(&repo.root);
    let before_status = repo.status();
    let ParseOutcome::Invocation(invocation) =
        parse_args(["--json", "inspect", "context"]).unwrap()
    else {
        panic!("expected invocation")
    };

    let streams = super::super::public_context::project_with_limit(&context, &invocation, 0)
        .render(OutputMode::Json);
    assert_eq!(streams.exit_code, 70);
    assert!(streams.stdout.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&streams.stderr).unwrap();
    assert_eq!(value["schema_version"], "HarnessDiagnostic-v1");
    assert_eq!(
        value["diagnostic_id"],
        "successor_runtime_projection_failed"
    );
    let output = String::from_utf8(streams.stderr).unwrap();
    assert!(!output.contains(repo.root.to_str().unwrap()));
    assert!(!output.contains("repository_root"));
    assert!(!output.contains("worktree_root"));
    assert_eq!(tree(&repo.root), before_tree);
    assert_eq!(repo.status(), before_status);
}
