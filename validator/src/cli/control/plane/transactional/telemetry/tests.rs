use super::*;
use serde_json::json;
use std::time::Instant;

fn root(label: &str) -> std::path::PathBuf {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    root
}

#[test]
fn attach_records_pass_and_fail_claim_contracts() {
    let root = root("transactional-telemetry-pass");
    let receipt = root
        .canonicalize()
        .expect("canonical root")
        .join("validation_artifacts/cli/transaction-finalize.json");
    let mut pass = json!({"status":"pass","blocked_claim_classes":[]});

    attach(&root, &receipt, &mut pass, Instant::now()).expect("attach pass");

    assert_eq!(pass["why_failed"], "none");
    assert_eq!(pass["where_failed"], "none");
    assert_eq!(pass["next_repair"], "none");
    assert_eq!(
        pass["claim_impact"],
        "supports_transactional_finalization_only_no_update_goal_call"
    );
    assert_eq!(
        pass["observability"]["supported_claims"][0],
        "transactional_finalization_same_candidate"
    );
    assert_eq!(
        pass["receipt_observability_binding"]["command_receipt_path"],
        "validation_artifacts/cli/transaction-finalize.json"
    );

    let mut fail = json!({
        "status":"fail",
        "blocked_claim_classes":["release", "release"],
        "failure":{"observed_failures":["coverage stale", "registry missing", "final_packet stale", "other"]}
    });
    attach(&root, &receipt, &mut fail, Instant::now()).expect("attach fail");
    assert_eq!(fail["observability"]["failure_class"], "coverage_blocker");
    assert!(
        fail["why_failed"]
            .as_str()
            .unwrap()
            .contains("coverage stale")
    );
    assert_eq!(
        fail["claim_impact"],
        "blocks_transactional_finalization_readiness_release_completion_update_goal"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn failure_classification_edges_are_specific() {
    assert_eq!(failure_class("none"), "none");
    assert_eq!(failure_class("coverage stale"), "coverage_blocker");
    assert_eq!(failure_class("registry missing"), "registry_blocker");
    assert_eq!(failure_class("final_packet stale"), "final_packet_blocker");
    assert_eq!(
        failure_class("source audit stale"),
        "transactional_finalization_blocked"
    );

    assert_eq!(
        why_failed(&json!({"status":"fail"})),
        "transactional_finalization_not_proven"
    );
    assert!(blocked_claims(&json!({"blocked_claim_classes":"bad"})).is_empty());
    assert_eq!(
        relative_receipt_path(
            std::path::Path::new("/tmp/missing-transactional-root"),
            std::path::Path::new("receipt.json")
        ),
        None
    );

    let root = root("transactional-telemetry-relative");
    assert_eq!(
        relative_receipt_path(
            &root,
            std::path::Path::new("validation_artifacts/cli/transaction-finalize.json")
        ),
        Some("validation_artifacts/cli/transaction-finalize.json".to_string())
    );
    std::fs::remove_dir_all(root).expect("cleanup relative");
}

#[test]
fn attach_propagates_observability_spool_write_errors() {
    let root = root("transactional-telemetry-spool-error");
    std::fs::write(root.join("validation_artifacts"), b"not a directory").expect("block spool dir");
    let mut value = json!({"status":"fail"});

    let error = attach(
        &root,
        &root.join("validation_artifacts/cli/transaction-finalize.json"),
        &mut value,
        Instant::now(),
    )
    .expect_err("spool write fails");

    assert!(
        error.contains("validation_artifacts/observability/spool"),
        "{error}"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn attach_marks_outside_root_receipts_instead_of_leaking_private_paths() {
    let root = root("transactional-telemetry-outside-receipt");
    let mut value = json!({"status":"pass","blocked_claim_classes":[]});
    let outside_receipt = std::path::PathBuf::from(std::path::MAIN_SEPARATOR.to_string())
        .join("private")
        .join("tmp")
        .join("outside-transaction-finalize.json");

    attach(&root, &outside_receipt, &mut value, Instant::now()).expect("attach outside receipt");

    assert_eq!(
        value["receipt_observability_binding"]["command_receipt_path"],
        "<outside-root-receipt>"
    );
    std::fs::remove_dir_all(root).expect("cleanup outside receipt");
}
