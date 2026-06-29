use crate::cli::observe;
use crate::cli::observe::types::ObserveOperation;
use serde_json::json;
use std::fs;

mod package;
mod query;
mod run;

pub(super) fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

#[test]
fn observe_parser_covers_required_command_inventory() {
    let cases = [
        (&["observe", "stack", "up"][..], ObserveOperation::StackUp),
        (
            &["observe", "stack", "health"][..],
            ObserveOperation::StackHealth,
        ),
        (
            &["observe", "stack", "smoke"][..],
            ObserveOperation::StackSmoke,
        ),
        (
            &["observe", "stack", "down"][..],
            ObserveOperation::StackDown,
        ),
        (
            &["observe", "stack", "gc", "plan"][..],
            ObserveOperation::StackGcPlan,
        ),
        (
            &["observe", "stack", "gc", "dry-run"][..],
            ObserveOperation::StackGcDryRun,
        ),
        (
            &["observe", "stack", "gc", "apply"][..],
            ObserveOperation::StackGcApply,
        ),
        (
            &["observe", "logs", "query"][..],
            ObserveOperation::LogsQuery,
        ),
        (
            &["observe", "metrics", "query"][..],
            ObserveOperation::MetricsQuery,
        ),
        (
            &["observe", "traces", "query"][..],
            ObserveOperation::TracesQuery,
        ),
        (&["observe", "snapshot"][..], ObserveOperation::Snapshot),
        (&["observe", "prove"][..], ObserveOperation::Prove),
        (
            &["observe", "explain-failure"][..],
            ObserveOperation::ExplainFailure,
        ),
        (
            &["observe", "explain-claim"][..],
            ObserveOperation::ExplainClaim,
        ),
        (
            &["observe", "explain-check"][..],
            ObserveOperation::ExplainCheck,
        ),
        (
            &["observe", "explain-law"][..],
            ObserveOperation::ExplainLaw,
        ),
    ];
    for (raw, operation) in cases {
        let parsed = observe::parse(&args(raw))
            .expect("parse")
            .expect("observe command");
        assert_eq!(parsed.operation, operation);
    }
}

#[test]
fn observe_receipt_blocks_completion_when_live_stack_is_not_proven() {
    let root = minimal_root("observe-receipt");
    let command = observe::parse(&args(&["observe", "prove"]))
        .expect("parse")
        .expect("observe command");
    let receipt = observe::telemetry::prove(&root, &command).expect("receipt");
    assert_eq!(receipt["status"], "fail");
    assert_eq!(
        receipt["claim_ceiling"],
        "observability_gate_failed_completion_readiness_release_update_goal_blocked"
    );
    assert!(
        receipt["blocked_claims"]
            .as_array()
            .expect("blocked")
            .iter()
            .any(|claim| claim.as_str() == Some("update_goal_eligibility"))
    );
    assert!(
        root.join("validation_artifacts/observability/spool/events.jsonl")
            .is_file()
    );
    fs::remove_dir_all(root).expect("cleanup observe receipt");
}

#[test]
fn observe_query_rejects_unbounded_requests() {
    let root = minimal_root("observe-query");
    let command = observe::parse(&args(&[
        "observe",
        "logs",
        "query",
        "--timeout-ms",
        "0",
        "--limit",
        "0",
    ]))
    .expect("parse")
    .expect("observe command");
    let result = observe::query::run(&root, &command).expect("query result");
    assert_eq!(result["status"], "fail");
    assert_eq!(result["failure"], "unbounded observability query rejected");
    fs::remove_dir_all(root).expect("cleanup observe query");
}

pub(super) fn minimal_root(label: &str) -> std::path::PathBuf {
    let root = crate::self_tests::boundaries::support::temp_root(label);
    fs::create_dir_all(&root).expect("root");
    fs::write(root.join("owned.txt"), "owned").expect("owned");
    let manifest = json!({"resources":["owned.txt"]});
    crate::json_boundary::write_json(&root.join("plugin-manifest-draft.json"), &manifest)
        .expect("manifest");
    root
}
