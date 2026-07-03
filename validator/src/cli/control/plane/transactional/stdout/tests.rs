use super::lines;
use serde_json::json;
use std::path::Path;

#[test]
fn pass_summary_uses_explicit_supported_claims_only() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("transactional-stdout-pass");
    std::fs::create_dir_all(&root).expect("root");
    let receipt = root
        .canonicalize()
        .expect("canonical root")
        .join("validation_artifacts/cli/transaction-finalize.json");
    let value = json!({
        "status": "pass",
        "candidate_digest": "sha256:test",
        "run_id": "run-test",
        "correlation_id": "corr-test",
        "claim_impact": "supports_transactional_finalization_only_no_update_goal_call",
        "observability": {
            "supported_claims": ["transactional_finalization_same_candidate"],
            "blocked_claims": ["update_goal"]
        }
    });

    let output = lines(&root, &receipt, &value);

    assert_eq!(output.len(), 1);
    assert!(output[0].contains("proven=transactional_finalization_same_candidate"));
    assert!(output[0].contains("receipt=validation_artifacts/cli/transaction-finalize.json"));
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn pass_summary_without_supported_claims_does_not_overstate_finalization() {
    let output = lines(
        Path::new("/tmp/nonexistent-transaction-root"),
        Path::new("outside.json"),
        &json!({
            "status": "pass",
            "candidate_digest": "sha256:test",
            "run_id": "run-test",
            "correlation_id": "corr-test",
            "claim_impact": "missing_supported_claims"
        }),
    );

    assert_eq!(output.len(), 1);
    assert!(output[0].contains("proven=none"), "{}", output[0]);
    assert!(output[0].contains("receipt=<outside-root-receipt>"));
}

#[test]
fn failure_output_contains_query_hints_and_bound_receipt_path() {
    let value = json!({
        "status": "fail",
        "candidate_digest": "sha256:test",
        "run_id": "run-test",
        "correlation_id": "corr-test",
        "law_id": "transaction-law",
        "check_id": "transaction-check",
        "why_failed": "coverage stale",
        "where_failed": "transaction_finalize#/failure/observed_failures/0",
        "next_repair": "rerun exact coverage",
        "claim_impact": "blocks_update_goal",
        "receipt_observability_binding": {
            "command_receipt_path": "validation_artifacts/cli/transaction-finalize.json"
        }
    });

    let output = lines(Path::new("."), Path::new("ignored.json"), &value);

    assert_eq!(output.len(), 2);
    assert!(output[0].contains("proven=none"));
    assert!(output[1].contains("failed_law=transaction-law"));
    assert!(output[1].contains("query_logs='ultragoal observe logs query --run-id run-test"));
    assert!(output[1].contains("receipt=validation_artifacts/cli/transaction-finalize.json"));
}

#[test]
fn fallback_receipt_path_accepts_relative_receipt_inside_root() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "transactional-stdout-relative",
    );
    std::fs::create_dir_all(root.join("validation_artifacts/cli")).expect("receipt dir");
    let receipt = std::path::Path::new("validation_artifacts/cli/transaction-finalize.json");
    let value = json!({
        "status": "pass",
        "candidate_digest": "sha256:test",
        "run_id": "run-test",
        "correlation_id": "corr-test",
        "claim_impact": "source_local_only"
    });

    let output = lines(&root, receipt, &value);

    assert!(output[0].contains("receipt=validation_artifacts/cli/transaction-finalize.json"));
    std::fs::remove_dir_all(root).expect("cleanup relative receipt");
}
