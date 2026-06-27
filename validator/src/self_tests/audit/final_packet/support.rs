use serde_json::{Value, json};
use std::path::Path;

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

pub(crate) fn control_receipt(candidate: &str, operation: &str) -> Value {
    json!({
        "schema":"harness-ultragoal.cli-control-plane-receipt.v1",
        "issuer":{"tool":"ultragoal","authority":"cli_control_plane","self_law_state":"self_hosted"},
        "operation":operation,
        "candidate_digest":candidate,
        "status":"pass",
        "claim_ceiling":"supports_update_goal_eligibility",
        "blocked_claim_classes":[],
        "failure":null
    })
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
        &json!({"raw":true}),
    );
    let receipt = json!({
        "schema": "harness-ultragoal.final-packet-proof.v1",
        "generated_at": "2026-06-27T00:00:00Z",
        "status": "pass",
        "target_revision": {"kind": "package_digest", "value": current},
        "packet": {"path": packet_path, "digest": crate::digest::file(&root.join(packet_path)).expect("packet digest")},
        "cli_update_goal": control_ref(root, current, "update-goal-eligibility", "update_goal_eligibility"),
        "cli_self_law": control_ref(root, current, "self-law-receipt", "self_update_goal_eligibility"),
        "cli_performance": performance_ref(root, current),
        "registry_exposure": registry_ref(root, current, &raw),
        "source_audit": source_audit_ref(root, current),
        "coverage": coverage_ref(root, current),
        "package_receipts": [package_ref(root, current)],
        "claim_ceiling": "final_packet_claims_cli_verified"
    });
    write_proof(root, &receipt);
    receipt
}

fn ref_for(root: &Path, rel: &str, value: &Value) -> Value {
    write_json(&root.join(rel), value);
    json!({
        "path": rel,
        "digest": crate::digest::file(&root.join(rel)).expect("ref digest"),
        "status": "pass"
    })
}

fn control_ref(root: &Path, candidate: &str, name: &str, operation: &str) -> Value {
    ref_for(
        root,
        &format!("validation_artifacts/cli/{name}.json"),
        &control_receipt(candidate, operation),
    )
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

fn source_audit_ref(root: &Path, current: &str) -> Value {
    ref_for(
        root,
        "validation_artifacts/ultragoal-audit/validator-receipt.json",
        &json!({"status":"pass","target_revision":{"kind":"package_digest","value":current}}),
    )
}

fn coverage_ref(root: &Path, current: &str) -> Value {
    ref_for(
        root,
        "validation_artifacts/coverage/coverage-receipt.json",
        &json!({
            "schema":"harness-ultragoal.coverage-receipt.v1",
            "claim_id":"CLAIM-100",
            "command":"ultragoal coverage prove",
            "tool":"cargo-llvm-cov",
            "source_tree_digest":crate::self_tests::boundaries::support::sha('2'),
            "coverage_manifest_digest":crate::self_tests::boundaries::support::sha('3'),
            "coverage_command_digest":crate::self_tests::boundaries::support::sha('4'),
            "changed_files_digest":crate::self_tests::boundaries::support::sha('5'),
            "tool_version":"test",
            "workspace_root":".",
            "target_revision":{"kind":"package_digest","value":current},
            "command_started_at":"2026-06-27T00:00:00Z",
            "command_completed_at":"2026-06-27T00:00:01Z",
            "command_exit":0,
            "machine_readable_report":{"path":"validation_artifacts/coverage/report.json","digest":crate::self_tests::boundaries::support::sha('6')},
            "generated_by":"coverage-command",
            "percent_source":"machine_readable_report",
            "target_paths":["validator/src"],
            "measured_dimensions":["line"],
            "coverage":{"percent":100.0,"floor_percent":100.0,"policy":"100_percent_required"},
            "uncovered_records":[],
            "exclusions":[],
            "generated_at":"2026-06-27T00:00:01Z",
            "claim_ceiling":"supports_complete_claim"
        }),
    )
}

fn package_ref(root: &Path, current: &str) -> Value {
    ref_for(
        root,
        "validation_artifacts/harness/package-receipt.json",
        &json!({"status":"pass","target_revision":{"kind":"package_digest","value":current}}),
    )
}
