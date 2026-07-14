use crate::cli::observe::command::{ObserveCommand, ObserveOperation};
use serde_json::json;
use std::path::PathBuf;

mod bounds_redaction;
mod failure_reconciliation;
mod failures;
mod identity;
mod proof;
mod reconciliation;
mod runner;
mod selection;
mod spec_binding;
mod tamper;

fn command() -> ObserveCommand {
    ObserveCommand {
        operation: ObserveOperation::CommandRoundtrip,
        receipt: None,
        query: None,
        run_id: None,
        correlation_id: None,
        claim_id: None,
        check_id: None,
        law_id: None,
        target_command: Some("package-digest".to_string()),
        target_family: None,
        row_limit: 100,
        byte_limit: 1024,
        timeout_ms: 1000,
    }
}

fn roundtrip_root(label: &str) -> (PathBuf, String) {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
    std::fs::create_dir_all(&root).expect("root");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let event = json!({
        "schema": crate::cli::observe::command::EVENT_SCHEMA,
        "run_id": "run-fit",
        "correlation_id": "corr-fit",
        "candidate_digest": candidate,
        "operation": "package.digest",
        "status": "pass",
        "failure_class": "none",
        "why_failed": "none",
        "where_failed": "none",
        "next_repair": "none",
        "claim_impact": "supports_source_package_digest_only",
        "law_id": crate::cli::observe::command::LAW_ID,
        "check_id": "package-digest-observability-binding",
        "claim_id": "source_package_digest"
    });
    crate::cli::observe::telemetry::spool_write_for_test(&root, &event).expect("target event");
    crate::json_boundary::write_json(
        &root.join("validation_artifacts/observability/package-digest.json"),
        &json!({
            "schema": crate::cli::observe::command::RECEIPT_SCHEMA,
            "run_id": "run-fit",
            "correlation_id": "corr-fit",
            "candidate_digest": candidate,
            "status": "pass",
            "event": event
        }),
    )
    .expect("command receipt");
    (root, candidate)
}
