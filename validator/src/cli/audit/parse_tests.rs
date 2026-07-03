use super::*;
use serde_json::json;
use std::fs;

#[test]
fn source_audit_parse_rejection_writes_observability() {
    let root = audit_parse_root("source-audit-parse-observe");
    let err = crate::parse_args_from(args(&[
        "--root",
        root.to_str().expect("root path"),
        "source",
        "audit",
        "--receipt",
        "validation_artifacts/ultragoal-audit/validator-receipt.json",
        "--mode",
        "not-a-mode",
    ]))
    .expect_err("source audit parse rejection");
    assert!(err.contains("invalid source audit --mode"), "{err}");
    let value =
        crate::json_boundary::read_json(&root.join(observability::SOURCE_RECEIPT)).expect("obs");
    assert_parse_rejection(&value, "source.audit", "source.audit.parse");
    assert_eq!(value["claim_id"], "source_audit");
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn target_repo_audit_parse_rejection_writes_target_observability() {
    let root = audit_parse_root("target-audit-parse-observe");
    let err = crate::parse_args_from(args(&[
        "--root",
        root.to_str().expect("root path"),
        "target-repo",
        "audit",
        "--receipt",
        "validation_artifacts/ultragoal-audit/target-receipt.json",
    ]))
    .expect_err("target audit parse rejection");
    assert!(
        err.contains("missing required argument --surface-root"),
        "{err}"
    );
    let value = crate::json_boundary::read_json(
        &root.join("validation_artifacts/observability/target-repo-audit.json"),
    )
    .expect("target obs");
    assert_parse_rejection(&value, "target-repo.audit", "target-repo.audit.parse");
    assert_eq!(value["claim_id"], "target_repo_audit");
    fs::remove_dir_all(root).expect("cleanup");
}

fn assert_parse_rejection(value: &serde_json::Value, operation: &str, where_failed: &str) {
    assert_eq!(value["status"], "fail");
    assert_eq!(value["operation"], operation);
    assert_eq!(value["event"]["failure_class"], "audit_parse_rejection");
    assert_eq!(value["event"]["where_failed"], where_failed);
    assert_eq!(value["event"]["worker_count"], 0);
    assert_eq!(value["event"]["task_count"], 0);
    assert_eq!(value["event"]["queue_depth"], 0);
    assert_eq!(value["event"]["cache_mode"], "parser_rejection_no_cache");
    assert_eq!(
        value["event"]["resource_measurement_status"],
        "parse_rejected_before_scheduler_metrics"
    );
    assert_eq!(value["event"]["repair_anchor_before"], "audit_parse_start");
    assert_eq!(
        value["event"]["repair_anchor_after"],
        "audit_parse_rejection_telemetry_emit"
    );
    assert!(
        value["blocked_claims"]
            .as_array()
            .expect("blocked")
            .iter()
            .any(|item| item.as_str() == Some("update_goal_eligibility"))
    );
}

fn audit_parse_root(name: &str) -> std::path::PathBuf {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(name);
    fs::create_dir_all(&root).expect("root");
    fs::write(root.join("owned.txt"), "owned").expect("owned");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["owned.txt"]}),
    )
    .expect("manifest");
    root
}

fn args(raw: &[&str]) -> Vec<String> {
    raw.iter().map(|arg| (*arg).to_string()).collect()
}
