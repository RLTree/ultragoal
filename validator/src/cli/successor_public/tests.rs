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
