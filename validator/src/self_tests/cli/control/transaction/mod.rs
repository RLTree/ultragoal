use serde_json::{Value, json};
use std::path::Path;

const RECEIPT: &str = "validation_artifacts/cli/transactional-finalization-receipt.json";
const SCHEMA: &str = "harness-ultragoal.cli-transactional-finalization-receipt.v1";

mod reference;
mod required;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn write_manifest(root: &Path) {
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"version":"0.0.0-test","resources":[]}),
    );
}

fn write_receipt_set(root: &Path, current: &str) {
    for (rel, value) in [
        (
            "validation_artifacts/review/final-packet-proof.json",
            json!({"status":"fail","target_revision":{"kind":"package_digest","value":current},"claim_ceiling":"withheld"}),
        ),
        (
            "validation_artifacts/ultragoal-audit/validator-receipt.json",
            json!({"schema":"harness-ultragoal.validator-receipt.v1","status":"fail","target_revision":{"kind":"package_digest","value":current}}),
        ),
        (
            "validation_artifacts/ultragoal-audit/active-registry-exposure-current.json",
            json!({"status":"fail","target_revision":{"value":current}}),
        ),
        (
            "validation_artifacts/cli/performance-receipt.json",
            performance_overclaim(current),
        ),
        (
            "validation_artifacts/coverage/coverage-receipt.json",
            json!({"schema":"harness-ultragoal.coverage-receipt.v1","target_revision":{"kind":"package_digest","value":current},"coverage":{"percent":99.0},"uncovered_records":[{"path":"validator/src/lib.rs"}]}),
        ),
    ] {
        write_json(&root.join(rel), &value);
    }
}

fn performance_overclaim(current: &str) -> Value {
    json!({
        "schema":"harness-ultragoal.cli-performance-receipt.v1",
        "status":"pass",
        "command":{"argv":["ultragoal"]},
        "budget":{
            "class":"strict_local",
            "cold_p95_ms":60000,
            "warm_p95_ms":null,
            "target_ms":60000,
            "hard_ceiling_ms":180000,
            "threshold_ms":60000
        },
        "digests":{"candidate":current},
        "cache":{"mode":"enabled","no_cache_mode_result":"used_cache"},
        "concurrency":{"worker_count":1},
        "telemetry":{"wall_clock_ms":1},
        "performance_regression":{"status":"pass"},
        "failure":null,
        "claim_ceiling":"performance_proven",
        "blocked_claim_classes":[],
        "supported_claim_classes":["update_goal_eligibility"]
    })
}

fn missing_ref(rel: &str) -> Value {
    json!({"path": rel, "digest": crate::digest::ZERO, "status": "pass"})
}

fn write_transaction(root: &Path, current: &str) {
    write_json(
        &root.join(RECEIPT),
        &json!({
            "schema": SCHEMA,
            "generated_at": "2026-06-27T00:00:00Z",
            "status": "pass",
            "candidate_digest": current,
            "claim_ceiling": "supports_update_goal_eligibility",
            "transaction_mode": "same_candidate_atomic_finalization",
            "blocked_claim_classes": [],
            "failure": null,
            "final_packet": ref_row(root, "validation_artifacts/review/final-packet-proof.json"),
            "registry_exposure": ref_row(root, "validation_artifacts/ultragoal-audit/active-registry-exposure-current.json"),
            "cli_performance": ref_row(root, "validation_artifacts/cli/performance-receipt.json"),
            "coverage": ref_row(root, "validation_artifacts/coverage/coverage-receipt.json")
        }),
    );
}

fn ref_row(root: &Path, rel: &str) -> Value {
    json!({
        "path": rel,
        "digest": crate::digest::file(&root.join(rel)).expect("digest"),
        "status": "pass"
    })
}

#[test]
fn transaction_finalize_command_writes_fail_closed_receipt_for_missing_refs() {
    let root = crate::self_tests::boundaries::support::temp_root("transaction-command-fail");
    write_manifest(&root);
    let receipt = root.join(RECEIPT);
    let code = crate::command_run::run_with_exit_code(crate::Args {
        root: root.clone(),
        command: crate::Command::TransactionalFinalization {
            receipt: receipt.clone(),
        },
    })
    .expect("transaction command");
    assert_eq!(code, 1);
    let value = crate::json_boundary::read_json(&receipt).expect("receipt json");
    let current = crate::package::inventory::package_digest(&root).expect("digest");
    assert_eq!(value["schema"], SCHEMA);
    assert_eq!(value["status"], "fail");
    assert_eq!(value["candidate_digest"], current);
    assert_eq!(value["claim_ceiling"], "withheld_or_blocked");
    assert!(
        value["failure"]["observed_failures"]
            .as_array()
            .expect("failures")
            .iter()
            .any(|failure| failure
                .as_str()
                .is_some_and(|text| text.contains("final_packet"))),
        "{value}"
    );
    let failures = crate::cli::control::plane::proof::failures(
        &root,
        crate::cli::control::plane::types::ControlOperation::UpdateGoalEligibility,
    );
    assert!(
        !failures
            .iter()
            .any(|failure| failure.contains("transactional_finalization_missing")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure == "cli_control_plane_transaction_status_not_pass"),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup transaction command fail");
}

#[test]
fn transaction_references_use_typed_receipt_status_not_file_existence() {
    let root = crate::self_tests::boundaries::support::temp_root("transaction-typed-refs");
    write_manifest(&root);
    let current = crate::package::inventory::package_digest(&root).expect("digest");
    write_json(
        &root.join("validation_artifacts/review/final-packet-proof.json"),
        &json!({
            "schema":"harness-ultragoal.final-packet-proof.v1",
            "status":"fail",
            "target_revision":{"kind":"package_digest","value":current},
            "claim_ceiling":"withheld_or_blocked"
        }),
    );
    write_json(
        &root.join("validation_artifacts/ultragoal-audit/active-registry-exposure-current.json"),
        &json!({
            "schema":"harness-ultragoal.multi-agent-registry-exposure.v1",
            "status":"fail",
            "target_revision":{"kind":"package_digest","value":current},
            "claim_ceiling":"withheld_or_blocked"
        }),
    );
    write_json(
        &root.join("validation_artifacts/cli/performance-receipt.json"),
        &performance_overclaim(&current),
    );
    write_json(
        &root.join("validation_artifacts/coverage/coverage-receipt.json"),
        &json!({
            "schema":"harness-ultragoal.coverage-receipt.v1",
            "command_exit":0,
            "coverage":{"percent":100.0},
            "uncovered_records":[],
            "claim_ceiling":"supports_complete_coverage_claim",
            "supported_claim_classes":["complete_coverage"],
            "blocked_claim_classes":[
                "completion",
                "package_readiness",
                "review_readiness",
                "release_readiness",
                "final_packet_correctness",
                "update_goal_eligibility",
                "app_registry_or_reviewer_exposure"
            ],
            "target_revision":{"kind":"package_digest","value":current}
        }),
    );
    let value = crate::cli::control::plane::transactional::receipt(&root).expect("receipt");
    assert_eq!(value["status"], "fail");
    assert_eq!(value["final_packet"]["status"], "fail");
    assert_eq!(value["registry_exposure"]["status"], "fail");
    assert_eq!(value["coverage"]["status"], "pass");
    let failures = value["failure"]["observed_failures"]
        .as_array()
        .expect("observed failures");
    assert!(failures.iter().any(|failure| {
        failure
            .as_str()
            .is_some_and(|text| text.contains("ref_status_not_pass:final_packet"))
    }));
    assert!(failures.iter().any(|failure| {
        failure
            .as_str()
            .is_some_and(|text| text.contains("ref_status_not_pass:registry_exposure"))
    }));
    std::fs::remove_dir_all(root).expect("cleanup transaction typed refs");
}
