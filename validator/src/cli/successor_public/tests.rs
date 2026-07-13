use super::test_support::{Repository, tree};
use super::{
    MAX_PUBLIC_OUTPUT, execute_invocation, parse_public, public_output_allowed, read_context,
};
use crate::cli::successor::{OutputMode, ParseOutcome, parse_args};
use crate::observability::{EventStore, SemanticEvent};
use std::fs;
use std::path::Path;

#[test]
fn public_output_limit_is_inclusive_and_fail_closed() {
    assert!(public_output_allowed(MAX_PUBLIC_OUTPUT));
    assert!(!public_output_allowed(MAX_PUBLIC_OUTPUT + 1));
}

#[test]
fn public_router_adopts_only_live_successor_authority() {
    for args in [
        vec!["next"],
        vec!["--json", "inspect", "inventory"],
        vec!["package", "build", "--output", "out.json"],
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
    for args in [
        vec!["--json", "package", "inventory"],
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
        let failure = parse_public(&raw).expect_err("legacy route must fail closed");
        assert!(failure.contains("harness-ultragoal.cli-error.v1"));
    }
}

#[test]
fn fit_read_routes_are_public_and_zero_write() {
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
fn fit_target_option_selects_the_context_root_instead_of_being_ignored() {
    let ParseOutcome::Invocation(invocation) =
        parse_args(["--json", "fit", "inspect", "--target", "nested"]).unwrap()
    else {
        panic!("expected fit invocation")
    };
    let base = Path::new("/tmp/hul-fit-public-target-root");
    assert_eq!(
        super::fit::target_root(base, &invocation).unwrap(),
        base.join("nested")
    );
}

#[test]
fn fit_apply_remains_unavailable_without_root_effect_authority() {
    let repo = Repository::new("fit-apply-unavailable");
    let before_tree = tree(&repo.root);
    let before_status = repo.status();
    let ParseOutcome::Invocation(invocation) = parse_args([
        "--json",
        "fit",
        "apply",
        "--plan",
        "plan.json",
        "--accept-plan",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    ])
    .unwrap() else {
        panic!("expected fit apply invocation")
    };
    let streams = execute_invocation(&repo.root, invocation).render(OutputMode::Json);
    assert_eq!(streams.exit_code, 4);
    assert!(streams.stdout.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&streams.stderr).unwrap();
    assert_eq!(
        value["diagnostic_id"],
        "successor_runtime_downstream_tool_unavailable"
    );
    assert_eq!(tree(&repo.root), before_tree);
    assert_eq!(repo.status(), before_status);
}

#[test]
fn accepted_observe_query_reads_current_local_events_without_writes() {
    let repo = Repository::new("observe-query");
    let spool = repo.root.join("validation_artifacts/observability/spool");
    fs::create_dir_all(&spool).unwrap();
    let context = read_context(&repo.root).unwrap();
    let store = EventStore::for_context(
        super::observe::store_path(&repo.root),
        &context,
        "successor-runtime",
    )
    .unwrap();
    let matching = SemanticEvent::for_context(
        &context,
        "successor-runtime",
        "event-matching",
        1,
        1,
        "check.routine",
        "fail",
    )
    .unwrap();
    let other = SemanticEvent::for_context(
        &context,
        "successor-runtime",
        "event-other",
        2,
        2,
        "fit.verify",
        "pass",
    )
    .unwrap();
    assert!(store.append(&matching).unwrap());
    assert!(store.append(&other).unwrap());
    context.revalidate().unwrap();

    let before_tree = tree(&repo.root);
    let before_status = repo.status();
    let ParseOutcome::Invocation(invocation) =
        parse_args(["--json", "observe", "query", "--filter", "check.routine"]).unwrap()
    else {
        panic!("expected invocation")
    };
    let streams = execute_invocation(&repo.root, invocation).render(OutputMode::Json);
    assert_eq!(streams.exit_code, 0);
    assert!(streams.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&streams.stdout).unwrap();
    assert_eq!(value["schema_version"], "ObservabilityQuery-v1");
    assert_eq!(value["store_status"], "available");
    assert_eq!(value["event_count"], 1);
    assert_eq!(value["events"][0]["operation"], "check.routine");
    assert_eq!(value["claim_effect"], "none");
    assert_eq!(tree(&repo.root), before_tree);
    assert_eq!(repo.status(), before_status);
}

#[test]
fn absent_observe_store_is_empty_and_zero_write() {
    let repo = Repository::new("observe-absent");
    let before_tree = tree(&repo.root);
    let before_status = repo.status();
    let ParseOutcome::Invocation(invocation) = parse_args(["--json", "observe", "query"]).unwrap()
    else {
        panic!("expected invocation")
    };
    let streams = execute_invocation(&repo.root, invocation).render(OutputMode::Json);
    assert_eq!(streams.exit_code, 0);
    let value: serde_json::Value = serde_json::from_slice(&streams.stdout).unwrap();
    assert_eq!(value["store_status"], "absent");
    assert_eq!(value["event_count"], 0);
    assert_eq!(value["causal_status"], "not_evaluated");
    assert_eq!(tree(&repo.root), before_tree);
    assert_eq!(repo.status(), before_status);
}

#[cfg(unix)]
#[test]
fn substituted_observe_store_parent_fails_closed_without_path_echo() {
    let repo = Repository::new("observe-symlink");
    let outside = repo.root.with_extension("outside-observe");
    fs::create_dir_all(repo.root.join("validation_artifacts/observability")).unwrap();
    fs::create_dir_all(&outside).unwrap();
    std::os::unix::fs::symlink(
        &outside,
        repo.root.join("validation_artifacts/observability/spool"),
    )
    .unwrap();
    let before_tree = tree(&repo.root);
    let before_status = repo.status();
    let ParseOutcome::Invocation(invocation) = parse_args(["--json", "observe", "query"]).unwrap()
    else {
        panic!("expected invocation")
    };
    let streams = execute_invocation(&repo.root, invocation).render(OutputMode::Json);
    assert_eq!(streams.exit_code, 4);
    assert!(streams.stdout.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&streams.stderr).unwrap();
    assert_eq!(
        value["diagnostic_id"],
        "successor_runtime_observability_unavailable"
    );
    assert!(!String::from_utf8_lossy(&streams.stderr).contains("outside-observe"));
    assert_eq!(tree(&repo.root), before_tree);
    assert_eq!(repo.status(), before_status);
    fs::remove_dir_all(outside).unwrap();
}

#[cfg(unix)]
#[test]
fn substituted_missing_store_ancestor_fails_closed_without_path_echo() {
    let repo = Repository::new("observe-ancestor-symlink");
    let outside = repo.root.with_extension("outside-observe-ancestor");
    fs::create_dir_all(repo.root.join("validation_artifacts")).unwrap();
    fs::create_dir_all(&outside).unwrap();
    std::os::unix::fs::symlink(
        &outside,
        repo.root.join("validation_artifacts/observability"),
    )
    .unwrap();
    let before_tree = tree(&repo.root);
    let before_status = repo.status();
    let ParseOutcome::Invocation(invocation) = parse_args(["--json", "observe", "query"]).unwrap()
    else {
        panic!("expected invocation")
    };
    let streams = execute_invocation(&repo.root, invocation).render(OutputMode::Json);
    assert_eq!(streams.exit_code, 4);
    assert!(streams.stdout.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&streams.stderr).unwrap();
    assert_eq!(
        value["diagnostic_id"],
        "successor_runtime_observability_unavailable"
    );
    assert!(!String::from_utf8_lossy(&streams.stderr).contains("outside-observe-ancestor"));
    assert_eq!(tree(&repo.root), before_tree);
    assert_eq!(repo.status(), before_status);
    fs::remove_dir_all(outside).unwrap();
}

#[test]
fn unavailable_export_route_fails_before_repository_access_or_effect() {
    let canary = Path::new("/missing/successor-observe-export-canary-1297");
    let ParseOutcome::Invocation(invocation) = parse_args([
        "--json",
        "observe",
        "export",
        "--output",
        "out.json",
        "--approve-export",
    ])
    .unwrap() else {
        panic!("expected invocation")
    };
    let streams = execute_invocation(canary, invocation).render(OutputMode::Json);
    assert_eq!(streams.exit_code, 3);
    assert!(streams.stdout.is_empty());
    let text = String::from_utf8(streams.stderr).unwrap();
    assert!(text.contains("successor_runtime_authority_required"));
    assert!(!text.contains("successor-observe-export-canary-1297"));
    assert!(!text.contains("out.json"));
}

#[test]
fn live_inventory_and_state_reads_are_zero_write() {
    let repo = Repository::new("reads");
    fs::write(repo.root.join("dirty-canary"), b"dirty\n").unwrap();
    let before_tree = tree(&repo.root);
    let before_status = repo.status();

    for args in [
        &["--json", "inspect", "inventory"][..],
        &["--json", "inspect"][..],
        &["--json", "next"][..],
    ] {
        let ParseOutcome::Invocation(invocation) = parse_args(args.iter().copied()).unwrap() else {
            panic!("expected invocation")
        };
        let streams = execute_invocation(&repo.root, invocation).render(OutputMode::Json);
        assert!(matches!(streams.exit_code, 0 | 1 | 3));
        assert!(streams.stderr.is_empty());
        let value: serde_json::Value = serde_json::from_slice(&streams.stdout).unwrap();
        assert!(
            value["schema_version"]
                .as_str()
                .is_some_and(|schema| schema.ends_with("-v1"))
        );
    }

    assert_eq!(tree(&repo.root), before_tree);
    assert_eq!(repo.status(), before_status);
}

#[test]
fn unavailable_context_is_stable_and_does_not_echo_input() {
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
fn public_context_uses_opaque_root_ids_and_is_recursively_zero_write() {
    let repo = Repository::new("public-context-roots");
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
    assert_eq!(value["schema_version"], "HarnessPublicContext-v1");
    for key in ["repository_root_id", "worktree_root_id"] {
        assert!(
            value["roots"][key]
                .as_str()
                .is_some_and(|id| id.starts_with("sha256:") && id.len() == 71)
        );
    }
    assert!(value["roots"].get("repository_root").is_none());
    assert!(value["roots"].get("worktree_root").is_none());
    assert!(!String::from_utf8_lossy(&streams.stdout).contains(repo.root.to_str().unwrap()));
    assert_eq!(tree(&repo.root), before_tree);
    assert_eq!(repo.status(), before_status);
}

#[test]
fn public_context_output_failure_never_falls_back_to_private_live_context() {
    let repo = Repository::new("public-context-output-failure");
    let context = read_context(&repo.root).unwrap();
    let before_tree = tree(&repo.root);
    let before_status = repo.status();
    let ParseOutcome::Invocation(invocation) =
        parse_args(["--json", "inspect", "context"]).unwrap()
    else {
        panic!("expected invocation")
    };

    let streams = super::public_context::project_with_limit(&context, &invocation, 0)
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
