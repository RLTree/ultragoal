use super::*;
use serde_json::json;

#[test]
fn audit_failure_summary_groups_stale_inventory_and_observability_roots() {
    let stale = json!({
        "checks": {
            "coverage": {
                "status": "fail",
                "details": "coverage_receipt_target_digest_mismatch; more"
            }
        }
    });
    assert!(
        audit_failure_summary(&stale, None, 1)
            .contains("root_group=stale_or_wrong_digest_evidence")
    );

    let inventory = json!({
        "checks": {
            "package": {"status": "fail", "details": "missing=[validator/src/lib.rs]"}
        }
    });
    assert!(
        audit_failure_summary(&inventory, None, 1)
            .contains("root_group=package_inventory_mismatch")
    );

    let observability = json!({
        "checks": {
            "gate-92": {
                "status": "fail",
                "details": "observability_command_telemetry_query_not_current"
            }
        }
    });
    assert!(
        audit_failure_summary(&observability, None, 1)
            .contains("root_group=observability_reconciliation_incomplete")
    );
}

#[test]
fn audit_stdout_contract_lists_claims_and_bounded_query_hints() {
    let lines = contract(&json!({
        "status": "fail",
        "operation": "source.audit",
        "candidate_digest": "sha256:test",
        "receipt_path": "validation_artifacts/observability/source-audit.json",
        "run_id": "run-audit",
        "correlation_id": "corr-audit",
        "claim_impact": "source_audit_failed_blocks_readiness",
        "supported_claims": [],
        "blocked_claims": ["completion", "readiness"],
        "law_id": "gate-92",
        "check_id": "source-audit-observability-binding",
        "why_failed": "coverage receipt stale",
        "where_failed": "source.audit",
        "next_repair": "rerun exact coverage"
    }));

    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("unsupported_claims=completion,readiness"));
    assert!(lines[1].contains("failed_law=gate-92"));
    assert!(lines[1].contains("query_metrics='ultragoal observe metrics query"));
}
