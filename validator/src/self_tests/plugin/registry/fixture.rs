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
            "arguments_digest": crate::self_tests::boundaries::workspace_fixtures::sha('1')
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
            "arguments_digest": crate::self_tests::boundaries::workspace_fixtures::sha('1')
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
                "final_packet_correctness",
                "completion",
                "update_goal_eligibility"
            ]
        }
    })
}

pub(crate) fn fail_closed_raw_observation(current: &str) -> Value {
    json!({
        "schema": "harness-ultragoal.registry-observation.v1",
        "status": "fail",
        "candidate_digest": current,
        "generated_at": "2026-06-27T00:00:00Z",
        "observed": "live same-surface registry proof unavailable",
        "probe": {
            "source": "ultragoal.registry_probe",
            "command": "ultragoal registry probe",
            "capture_method": "fail_closed_no_capability"
        },
        "unsupported_claims": [
            "active_registry_exposure",
            "reviewer_exposure",
            "review_readiness",
            "release_readiness",
            "completion",
            "update_goal_eligibility"
        ]
    })
}

fn agent_types() -> Vec<Value> {
    crate::agent_roles::CANONICAL_AGENT_ROLES
        .iter()
        .map(|role| {
        json!({
            "role": role.name,
            "agent_manifest_path": role.manifest_path,
            "agent_manifest_digest": crate::self_tests::boundaries::workspace_fixtures::sha('a'),
            "source_manifest_present": true,
            "sandbox_mode": "read-only",
            "disk_cache_synced": true,
            "global_toml_present": true,
            "runtime_metadata_status": "unavailable",
            "custom_agent_discovery_status": "unavailable",
            "exposed": false
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
            "final_packet_correctness",
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
