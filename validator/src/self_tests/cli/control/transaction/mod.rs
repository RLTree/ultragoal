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
        "budget":{"class":"fast"},
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
            "final_packet": ref_row(root, "validation_artifacts/review/final-packet-proof.json"),
            "source_audit": ref_row(root, "validation_artifacts/ultragoal-audit/validator-receipt.json"),
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
