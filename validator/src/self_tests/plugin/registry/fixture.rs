use serde_json::{Value, json};

pub(crate) fn live_registry_receipt(current: &str, raw_digest: &str) -> Value {
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

pub(crate) fn raw_observation(current: &str) -> Value {
    json!({
        "schema": "harness-ultragoal.registry-raw-observation.v1",
        "candidate_digest": current,
        "captured_at": "2026-06-27T00:00:00Z",
        "issuer": {"tool":"multi_agent_v1","authority":"tool_registry"},
        "tool_call": {
            "name": "multi_agent_v1.tool_registry",
            "call_id": "call-1",
            "arguments_digest": crate::self_tests::boundaries::support::sha('1')
        },
        "boundary": {"account_id": "acct", "workspace_id": "workspace", "session_id": "session"},
        "source": "multi_agent_v1.tool_registry",
        "registry_rows": agent_types()
    })
}

pub(crate) fn fail_closed_registry_receipt(current: &str, raw_digest: &str) -> Value {
    json!({
        "schema": "harness-ultragoal.multi-agent-registry-exposure.v1",
        "generated_at": "2026-06-27T00:00:00Z",
        "captured_at": "2026-06-27T00:00:00Z",
        "status": "fail",
        "issuer": {"tool":"ultragoal","authority":"cli_control_plane"},
        "tool_call": {
            "name": "ultragoal registry probe",
            "call_id": "local-fail-closed",
            "arguments_digest": crate::digest::ZERO
        },
        "capture_method": "fail_closed_no_capability",
        "boundary": {"account_id": "unavailable", "workspace_id": "unavailable", "session_id": "session"},
        "source": "ultragoal.registry_probe",
        "target_revision": {"kind":"package_digest","value":current},
        "claim_ceiling": "withheld_or_blocked",
        "session_id": "session",
        "round_id": "round",
        "raw_observation": {
            "path": "validation_artifacts/ultragoal-audit/active-registry-observation-current.json",
            "digest": raw_digest
        },
        "capability_gap": capability_gap(raw_digest),
        "agent_types": agent_types()
            .into_iter()
            .map(|mut row| {
                row["disk_cache_synced"] = json!(false);
                row["global_toml_present"] = json!(false);
                row["exposed"] = json!(false);
                row
            })
            .collect::<Vec<_>>(),
        "failure": {
            "reason": "live_registry_reviewer_exposure_not_proven",
            "observed": "same-surface registry proof unavailable",
            "blocked_claim_classes": [
                "app_registry_or_reviewer_exposure",
                "review_readiness",
                "release_readiness",
                "completion",
                "update_goal_eligibility"
            ]
        }
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

fn capability_gap(raw_digest: &str) -> Value {
    json!({
        "schema": "harness-ultragoal.capability-gap.v1",
        "id": "registry-reviewer-exposure-test",
        "source_artifact": {
            "path": "validation_artifacts/ultragoal-audit/active-registry-observation-current.json",
            "digest": raw_digest
        },
        "source_session_id": "session",
        "observed_at": "2026-06-27T00:00:00Z",
        "affected_workflow": "registry_probe",
        "affected_law_ids": [
            "capability-gap-extraction-harness-capability-promotion",
            "connector-capability-discovery",
            "distribution-sharing-surface-claim-separation"
        ],
        "affected_claim_ids": [
            "app_registry_or_reviewer_exposure",
            "review_readiness",
            "release_readiness",
            "completion",
            "update_goal_eligibility"
        ],
        "missing_capability_class": "live_same_surface_plugin_registry_or_reviewer_exposure",
        "owner_surface": "codex_desktop_plugin_registry",
        "blocked_package_surfaces": ["active_registry_exposure", "reviewer_exposure"],
        "deterministic_repair_target": "provide_live_tool_registry_query_or_keep_claims_blocked",
        "chosen_promotion_artifact": "validation_artifacts/ultragoal-audit/active-registry-exposure-current.json",
        "current_claim_ceiling": "withheld_or_blocked",
        "required_evidence": ["live same-surface tool registry query"],
        "disposition": "open_claim_blocked"
    })
}
