use super::super::checks::{is_command_observable, query_passed, same_candidate};
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
    assert!(is_command_observable(
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
    assert!(!is_command_observable(
        &failed_production,
        &json!({"status": "pass", "candidate_digest": candidate}),
        &pass_query,
        &pass_query,
        &pass_query,
        &explain
    ));
}

#[test]
fn failed_command_roundtrip_is_observable_when_agent_legible_and_same_candidate() {
    let candidate = "sha256:current";
    let run_id = "run-install";
    let correlation_id = "corr-install";
    let production = CommandOutput {
        status_success: false,
        exit_code: 1,
        stdout: [
            "ultragoal-surface-audit fail",
            "run_id=run-install",
            "correlation_id=corr-install",
            "failed_check=install-audit-observability-binding",
            "why=package_surface_digest_mismatch",
            "where=package_surface_audit#/failures/0",
            "next_repair=refresh installed package after source-local proof graph passes",
            "query_logs='ultragoal observe logs query --run-id run-install'",
            "query_metrics='ultragoal observe metrics query --run-id run-install'",
            "query_traces='ultragoal observe traces query --run-id run-install'",
        ]
        .join(" "),
        stderr: String::new(),
    };
    let command_receipt = json!({
        "status": "fail",
        "candidate_digest": candidate,
        "run_id": run_id,
        "correlation_id": correlation_id,
        "why_failed": "package_surface_digest_mismatch",
        "where_failed": "package_surface_audit#/failures/0",
        "next_repair": "refresh installed package after source-local proof graph passes",
        "claim_impact": "blocks_install_cache_parity_readiness_release_completion_update_goal"
    });
    let observed = json!({
        "status": "pass",
        "row_count": 1,
        "candidate_digest": candidate,
        "observed_why_failed": "package_surface_digest_mismatch",
        "observed_where_failed": "package_surface_audit#/failures/0",
        "observed_next_repair": "refresh installed package after source-local proof graph passes"
    });
    let metrics = json!({
        "status": "pass",
        "row_count": 1,
        "candidate_digest": candidate,
        "observed_why_failed": "bounded metric signal carries failure class only",
        "observed_where_failed": "install_audit",
        "observed_next_repair": "query logs and traces for the full repair hint"
    });

    assert!(is_command_observable(
        &production,
        &command_receipt,
        &observed,
        &metrics,
        &observed,
        &observed
    ));

    let opaque_stdout = CommandOutput {
        stdout: "failed; inspect receipt".to_string(),
        ..production
    };
    assert!(!is_command_observable(
        &opaque_stdout,
        &command_receipt,
        &observed,
        &metrics,
        &observed,
        &observed
    ));
}
