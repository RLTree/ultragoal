use super::super::checks::{is_fitted, query_passed, same_candidate};
use super::super::process::CommandOutput;
use serde_json::json;

#[test]
fn query_and_candidate_reconciliation_are_strict() {
    let candidate = "sha256:fit";
    let command_receipt = json!({"candidate_digest": candidate});
    let pass_query = json!({"status": "pass", "row_count": 1, "candidate_digest": candidate});
    let empty_query = json!({"status": "pass", "row_count": 0, "candidate_digest": candidate});
    let explain = json!({"status": "pass", "candidate_digest": candidate});
    assert!(query_passed(&pass_query));
    assert!(!query_passed(&empty_query));
    assert!(same_candidate(
        &command_receipt,
        &pass_query,
        &pass_query,
        &pass_query,
        &explain
    ));
    assert!(!same_candidate(
        &json!({}),
        &pass_query,
        &pass_query,
        &pass_query,
        &explain
    ));
    let wrong = json!({"status": "pass", "row_count": 1, "candidate_digest": "sha256:old"});
    assert!(!same_candidate(
        &command_receipt,
        &wrong,
        &pass_query,
        &pass_query,
        &explain
    ));
    let production = CommandOutput {
        status_success: true,
        exit_code: 0,
        stdout: "ok".to_string(),
        stderr: String::new(),
    };
    assert!(is_fitted(
        &production,
        &json!({"status": "pass", "candidate_digest": candidate}),
        &pass_query,
        &pass_query,
        &pass_query,
        &explain
    ));
    let failed_production = CommandOutput {
        status_success: false,
        exit_code: 1,
        stdout: String::new(),
        stderr: "failed".to_string(),
    };
    assert!(!is_fitted(
        &failed_production,
        &json!({"status": "pass", "candidate_digest": candidate}),
        &pass_query,
        &pass_query,
        &pass_query,
        &explain
    ));
}
