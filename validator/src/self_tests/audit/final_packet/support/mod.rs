use serde_json::{Value, json};
use std::path::Path;

mod coverage;
mod observability;
mod package_receipts;

pub(crate) fn write_json(path: &Path, value: &Value) {
    let parent = path.parent().expect("test JSON path has a parent");
    std::fs::create_dir_all(parent).expect("parent");
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("json write");
}

pub(crate) fn write_proof(root: &Path, value: &Value) {
    write_json(
        &root.join("validation_artifacts/review/final-packet-proof.json"),
        value,
    );
}

pub(crate) fn write_green_proof(root: &Path, current: &str) -> Value {
    let packet_path = "validation_artifacts/review/final-packet.json";
    write_json(
        &root.join(packet_path),
        &json!({"schema":"harness-ultragoal.review-packet-successor.v1"}),
    );
    let raw = ref_for(
        root,
        "validation_artifacts/ultragoal-audit/live-registry-raw.json",
        &registry_raw_observation(current),
    );
    let mut receipt = json!({
        "schema": "harness-ultragoal.final-packet-proof.v1",
        "generated_at": "2026-06-27T00:00:00Z",
        "status": "pass",
        "target_revision": {"kind": "package_digest", "value": current},
        "packet": {"path": packet_path, "exists": true, "digest": crate::digest::file(&root.join(packet_path)).expect("packet digest")},
        "cli_performance": performance_ref(root, current),
        "registry_exposure": registry_ref(root, current, &raw),
        "source_audit": super::source_audit::ref_for(root, current),
        "coverage": coverage::ref_for(root, current),
        "package_receipts": package_receipts::refs(root, current),
        "claim_ceiling": "final_packet_evidence_dereferenced",
        "blocked_claim_classes": [],
        "failure": Value::Null
    });
    observability::attach(root, &mut receipt, "pass", "none");
    write_proof(root, &receipt);
    receipt
}

pub(crate) fn write_fail_closed_proof(root: &Path, current: &str) -> Value {
    let packet_path = "validation_artifacts/review/final-packet.json";
    write_json(
        &root.join(packet_path),
        &json!({"schema":"harness-ultragoal.review-packet-successor.v1"}),
    );
    let mut receipt = json!({
        "schema": "harness-ultragoal.final-packet-proof.v1",
        "generated_at": "2026-06-27T00:00:00Z",
        "status": "fail",
        "target_revision": {"kind": "package_digest", "value": current},
        "packet": {"path": packet_path, "exists": true, "digest": crate::digest::file(&root.join(packet_path)).expect("packet digest")},
        "cli_performance": performance_ref(root, current),
        "registry_exposure": fail_closed_registry_ref(root, current),
        "source_audit": super::source_audit::ref_for(root, current),
        "coverage": coverage::ref_for(root, current),
        "package_receipts": package_receipts::refs(root, current),
        "claim_ceiling": "withheld_or_blocked",
        "blocked_claim_classes": [
            "completion",
            "package_readiness",
            "review_readiness",
            "release_readiness",
            "update_goal_eligibility",
            "app_registry_or_reviewer_exposure"
        ],
        "failure": {
            "reason": "final_packet_proof_not_proven",
            "observed_failures": ["live_registry_reviewer_exposure_not_proven"]
        }
    });
    observability::attach(
        root,
        &mut receipt,
        "fail",
        "live_registry_reviewer_exposure_not_proven",
    );
    write_proof(root, &receipt);
    receipt
}

fn ref_for(root: &Path, rel: &str, value: &Value) -> Value {
    ref_for_with_status(root, rel, value, "pass")
}

fn ref_for_with_status(root: &Path, rel: &str, value: &Value, status: &str) -> Value {
    write_json(&root.join(rel), value);
    json!({
        "path": rel,
        "digest": crate::digest::file(&root.join(rel)).expect("ref digest"),
        "status": status
    })
}

fn performance_ref(root: &Path, candidate: &str) -> Value {
    ref_for(
        root,
        "validation_artifacts/cli/performance-receipt.json",
        &json!({
            "schema":"harness-ultragoal.cli-performance-receipt.v1",
            "status":"pass",
            "claim_ceiling":"performance_proven",
            "command":{"argv":["ultragoal","performance","prove"]},
            "budget":{"class":"strict_local"},
            "digests":{"candidate":candidate},
            "cache":{"mode":"disabled","no_cache_mode_result":"executed_without_cache"},
            "concurrency":{"worker_count":1},
            "telemetry":{"wall_clock_ms":1},
            "performance_regression":{"status":"pass"},
            "failure":null,
            "blocked_claim_classes":[],
            "supported_claim_classes":["routine_usability"]
        }),
    )
}

fn registry_ref(root: &Path, current: &str, raw: &Value) -> Value {
    ref_for(
        root,
        "validation_artifacts/ultragoal-audit/active-registry-exposure-current.json",
        &json!({
            "schema":"harness-ultragoal.multi-agent-registry-exposure.v1",
            "generated_at":"2026-06-27T00:00:00Z",
            "captured_at":"2026-06-27T00:00:00Z",
            "status":"pass",
            "issuer":{"tool":"multi_agent_v1","authority":"tool_registry"},
            "tool_call":{"name":"multi_agent_v1.tool_registry","call_id":"call","arguments_digest":crate::self_tests::boundaries::support::sha('1')},
            "capture_method":"live_tool_registry_query",
            "boundary":{"account_id":"acct","workspace_id":"workspace","session_id":"session"},
            "source":"multi_agent_v1.tool_registry",
            "target_revision":{"kind":"package_digest","value":current},
            "claim_ceiling":"live_registry_reviewer_exposure_proven",
            "session_id":"session",
            "round_id":"round",
            "raw_observation":{"path":"validation_artifacts/ultragoal-audit/live-registry-raw.json","digest":raw["digest"]},
            "agent_types":agent_types()
        }),
    )
}

fn registry_raw_observation(current: &str) -> Value {
    json!({
        "schema": "harness-ultragoal.registry-raw-observation.v1",
        "candidate_digest": current,
        "captured_at": "2026-06-27T00:00:00Z",
        "issuer":{"tool":"multi_agent_v1","authority":"tool_registry"},
        "tool_call":{"name":"multi_agent_v1.tool_registry","call_id":"call","arguments_digest":crate::self_tests::boundaries::support::sha('1')},
        "boundary":{"account_id":"acct","workspace_id":"workspace","session_id":"session"},
        "source":"multi_agent_v1.tool_registry",
        "registry_rows": agent_types()
    })
}

fn fail_closed_registry_ref(root: &Path, current: &str) -> Value {
    ref_for_with_status(
        root,
        "validation_artifacts/ultragoal-audit/active-registry-exposure-current.json",
        &json!({
            "schema":"harness-ultragoal.multi-agent-registry-exposure.v1",
            "generated_at":"2026-06-27T00:00:00Z",
            "captured_at":"2026-06-27T00:00:00Z",
            "status":"fail",
            "issuer":{"tool":"ultragoal","authority":"cli_control_plane"},
            "tool_call":{"name":"ultragoal registry probe","call_id":"fail-closed","arguments_digest":crate::digest::ZERO},
            "capture_method":"fail_closed_no_capability",
            "boundary":{"account_id":"unavailable","workspace_id":"unavailable","session_id":"session"},
            "source":"ultragoal.registry_probe",
            "target_revision":{"kind":"package_digest","value":current},
            "claim_ceiling":"withheld_or_blocked",
            "session_id":"session",
            "round_id":"round",
            "raw_observation":{"path":"validation_artifacts/ultragoal-audit/active-registry-observation-current.json","digest":crate::digest::ZERO},
            "capability_gap": super::registry::capability::gap::record(crate::digest::ZERO),
            "agent_types":agent_types().into_iter().map(|mut row| {
                row["disk_cache_synced"] = json!(false);
                row["global_toml_present"] = json!(false);
                row["exposed"] = json!(false);
                row
            }).collect::<Vec<_>>(),
            "failure":{
                "reason":"live_registry_reviewer_exposure_not_proven",
                "observed":"same-surface registry proof unavailable",
                "blocked_claim_classes":[
                    "app_registry_or_reviewer_exposure",
                    "review_readiness",
                    "release_readiness",
                    "completion",
                    "update_goal_eligibility"
                ]
            }
        }),
        "fail",
    )
}

fn agent_types() -> Vec<Value> {
    [
        (
            "harness_contract_claim_falsifier",
            "contract_claim_falsifier",
            "custom-agents/harness-contract-claim-falsifier.toml",
        ),
        (
            "harness_orchestration_recovery_falsifier",
            "orchestration_recovery_falsifier",
            "custom-agents/harness-orchestration-recovery-falsifier.toml",
        ),
        (
            "harness_security_trust_boundary_falsifier",
            "security_trust_boundary_falsifier",
            "custom-agents/harness-security-trust-boundary-falsifier.toml",
        ),
        (
            "harness_product_simplicity_falsifier",
            "product_simplicity_falsifier",
            "custom-agents/harness-product-simplicity-falsifier.toml",
        ),
    ]
    .into_iter()
    .map(|(agent_type, persona, custom_agent_path)| {
        json!({"agent_type":agent_type,"persona":persona,"custom_agent_path":custom_agent_path,
            "disk_cache_synced":true,"global_toml_present":true,"exposed":true})
    })
    .collect()
}
