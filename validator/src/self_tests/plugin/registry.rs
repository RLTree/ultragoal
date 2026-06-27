use serde_json::{Value, json};
use std::path::Path;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

#[test]
fn active_registry_exposure_requires_live_same_surface_observation_provenance() {
    let root = crate::self_tests::boundaries::support::temp_root("plugin-registry-live-proof");
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    let current = crate::package::inventory::package_digest(&root).expect("digest");
    let raw_path = root.join("validation_artifacts/ultragoal-audit/live-registry-raw.json");
    write_json(
        &raw_path,
        &json!({"tool":"multi_agent_v1.tool_registry","candidate":current}),
    );
    let raw_digest = crate::digest::file(&raw_path).expect("raw digest");
    let receipt = live_registry_receipt(&current, &raw_digest);
    let store = crate::schema_catalog::load(&crate::self_tests::boundaries::support::repo_root());
    assert!(crate::audit::plugin::registry::value_failures(&root, &store, &receipt).is_empty());

    let mut forged = receipt.clone();
    forged["raw_observation"]["digest"] = json!(crate::self_tests::boundaries::support::sha('f'));
    let failures = crate::audit::plugin::registry::value_failures(&root, &store, &forged);
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("raw_observation_digest_mismatch")),
        "{failures:?}"
    );

    let mut wrong_class = receipt.clone();
    wrong_class["raw_observation"]["path"] =
        json!("validation_artifacts/review/live-registry-raw.json");
    let failures = crate::audit::plugin::registry::value_failures(&root, &store, &wrong_class);
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("raw_observation_path_invalid")),
        "{failures:?}"
    );

    let mut traversal = receipt;
    traversal["raw_observation"]["path"] =
        json!("validation_artifacts/ultragoal-audit/../review/final-packet.json");
    let failures = crate::audit::plugin::registry::value_failures(&root, &store, &traversal);
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("raw_observation_path_invalid")),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup live registry proof");
}

fn live_registry_receipt(current: &str, raw_digest: &str) -> Value {
    json!({
        "schema": "harness-ultragoal.multi-agent-registry-exposure.v1",
        "generated_at": "2026-06-27T00:00:00Z",
        "captured_at": "2026-06-27T00:00:00Z",
        "status": "pass",
        "issuer": {"tool":"multi_agent_v1","authority":"tool_registry"},
        "tool_call": {
            "name": "multi_agent_v1.tool_registry",
            "call_id": "call-1",
            "arguments_digest": crate::self_tests::boundaries::support::sha('1')
        },
        "capture_method": "live_tool_registry_query",
        "boundary": {"account_id": "acct", "workspace_id": "workspace", "session_id": "session"},
        "source": "multi_agent_v1.tool_registry",
        "target_revision": {"kind":"package_digest","value":current},
        "claim_ceiling": "live_registry_reviewer_exposure_proven",
        "session_id": "session",
        "round_id": "round",
        "raw_observation": {
            "path": "validation_artifacts/ultragoal-audit/live-registry-raw.json",
            "digest": raw_digest
        },
        "agent_types": agent_types()
    })
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
        json!({
            "agent_type": agent_type,
            "persona": persona,
            "custom_agent_path": custom_agent_path,
            "disk_cache_synced": true,
            "global_toml_present": true,
            "exposed": true
        })
    })
    .collect()
}
