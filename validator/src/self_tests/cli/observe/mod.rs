use crate::cli::observe;
use crate::cli::observe::types::ObserveOperation;
use serde_json::json;
use std::fs;

mod package;
mod prove;
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

#[test]
fn observe_receipts_redact_private_paths_before_spool_and_receipt_binding() {
    let root = minimal_root("observe-redaction");
    let private_receipt = format!("{}observe-redaction.json", private_temp_marker());
    let command = observe::parse(&args(&["observe", "prove", "--receipt", &private_receipt]))
        .expect("parse")
        .expect("observe command");
    let private_home = format!(
        "{}{}",
        concat!("unix://", "/", "Users/"),
        "terrynoblin/.docker/run/docker.sock"
    );
    let receipt = observe::telemetry::base_receipt(
        &root,
        &command,
        "fail",
        Some(&format!("docker socket {private_home} failed")),
    )
    .expect("receipt");
    assert_eq!(receipt["redaction_proof"], "pass");
    assert!(
        receipt["why_failed"]
            .as_str()
            .unwrap()
            .contains("[redacted-home-path]")
    );
    assert!(
        !receipt.to_string().to_ascii_lowercase().contains(&format!(
            "{}{}",
            concat!("/", "users/"),
            "terrynoblin"
        )),
        "{receipt}"
    );
    assert!(
        !receipt
            .to_string()
            .to_ascii_lowercase()
            .contains(private_temp_marker().as_str()),
        "{receipt}"
    );
    let spool =
        fs::read_to_string(root.join("validation_artifacts/observability/spool/events.jsonl"))
            .expect("spool");
    assert!(spool.contains("[redacted-home-path]"));
    assert!(!spool.to_ascii_lowercase().contains(&format!(
        "{}{}",
        concat!("/", "users/"),
        "terrynoblin"
    )));
    assert!(
        !spool
            .to_ascii_lowercase()
            .contains(private_temp_marker().as_str())
    );
    let private_temp_uri = format!("file://{}example.txt", private_temp_marker());
    assert_eq!(
        observe::telemetry::redacted_failure_for_test(&private_temp_uri),
        "[redacted-private-tmp-path]"
    );
    fs::remove_dir_all(root).expect("cleanup observe redaction");
}

#[test]
fn command_telemetry_redacts_private_paths_before_receipt_binding() {
    let root = minimal_root("observe-command-redaction");
    let private_receipt = format!("{}command-receipt.json", private_temp_marker());
    let private_artifact = format!("{}command-artifact", private_temp_marker());
    let why_failed = format!("receipt write failed at {private_receipt}");
    let where_failed = format!("{private_receipt}#/event");
    let next_repair = format!("rerun with repo-local receipt instead of {private_receipt}");
    let receipt = observe::telemetry::command_receipt(
        &root,
        observe::telemetry::CommandTelemetry {
            command: "ultragoal test",
            subcommand: "prove",
            operation: "test.prove",
            surface: "source",
            law_id: "full-local-observability-stack-integration-non-opaque-failure",
            check_id: "observability-redaction",
            claim_id: "source_local_observability_stack",
            artifact_path: &private_artifact,
            receipt_path: &private_receipt,
            status: "fail",
            failure_class: "observability_private_path_leak",
            why_failed: &why_failed,
            where_failed: &where_failed,
            next_repair: &next_repair,
            claim_impact: "source_local_observability_blocked",
            blocked_claims: vec!["readiness".to_string()],
            supported_claims: vec![],
            runtime: None,
            emit: false,
        },
    )
    .expect("command telemetry receipt");
    assert_eq!(receipt["redaction_proof"], "pass");
    assert!(receipt.to_string().contains("[redacted-private-tmp-path]"));
    assert!(
        !receipt
            .to_string()
            .to_ascii_lowercase()
            .contains(private_temp_marker().as_str()),
        "{receipt}"
    );
    fs::remove_dir_all(root).expect("cleanup observe command redaction");
}

fn private_temp_marker() -> String {
    ["", "private", "tmp", ""].join("/")
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
