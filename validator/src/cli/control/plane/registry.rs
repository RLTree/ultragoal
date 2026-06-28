use crate::cli::control::plane::types::ControlOperation;
use serde_json::json;
use std::path::Path;

const ACTIVE_RECEIPT: &str =
    "validation_artifacts/ultragoal-audit/active-registry-exposure-current.json";
const RAW_OBSERVATION: &str =
    "validation_artifacts/ultragoal-audit/active-registry-observation-current.json";

pub(crate) fn mint_fail_closed_if_needed(
    root: &Path,
    operation: ControlOperation,
    candidate: &str,
) -> Result<(), String> {
    if !matches!(
        operation,
        ControlOperation::RegistryProbe | ControlOperation::AppSurfaceProbe
    ) {
        return Ok(());
    }
    if existing_live_pass(root)? {
        return Ok(());
    }
    let now = crate::audit::clock::now_iso();
    let raw = json!({
        "schema": "harness-ultragoal.registry-observation.v1",
        "generated_at": now,
        "status": "fail",
        "candidate_digest": candidate,
        "probe": {
            "command": "ultragoal registry probe",
            "capture_method": "fail_closed_no_capability",
            "source": "ultragoal.registry_probe"
        },
        "observed": "no live same-surface Codex plugin registry or reviewer-exposure proof channel was available to this CLI run",
        "unsupported_claims": [
            "plugins_ui_visibility",
            "marketplace_publication",
            "install_button_success",
            "launcher_runtime_exposure",
            "active_registry_exposure",
            "reviewer_exposure",
            "review_readiness",
            "release_readiness",
            "update_goal_eligibility"
        ]
    });
    crate::json_boundary::write_json(&root.join(RAW_OBSERVATION), &raw)?;
    let raw_digest = crate::digest::file(&root.join(RAW_OBSERVATION))?;
    let receipt = json!({
        "schema": "harness-ultragoal.multi-agent-registry-exposure.v1",
        "generated_at": now,
        "captured_at": now,
        "status": "fail",
        "issuer": {"tool": "ultragoal", "authority": "cli_control_plane"},
        "tool_call": {
            "name": "ultragoal registry probe",
            "call_id": "local-fail-closed",
            "arguments_digest": crate::digest::ZERO
        },
        "capture_method": "fail_closed_no_capability",
        "boundary": {
            "account_id": "unavailable",
            "workspace_id": "unavailable",
            "session_id": "019f0024-e3a1-7ed0-949a-4c52bd825fb1"
        },
        "source": "ultragoal.registry_probe",
        "target_revision": {"kind": "package_digest", "value": candidate},
        "claim_ceiling": "withheld_or_blocked",
        "session_id": "019f0024-e3a1-7ed0-949a-4c52bd825fb1",
        "round_id": "source-compliance-hardening-2026-06-28-fail-closed",
        "raw_observation": {"path": RAW_OBSERVATION, "digest": raw_digest},
        "agent_types": fail_closed_agent_types(),
        "failure": {
            "reason": "live_registry_reviewer_exposure_not_proven",
            "observed": "same-surface registry/reviewer proof unavailable; disk source/install/cache proof is not accepted as a substitute",
            "blocked_claim_classes": [
                "app_registry_or_reviewer_exposure",
                "review_readiness",
                "release_readiness",
                "completion",
                "update_goal_eligibility"
            ]
        }
    });
    crate::json_boundary::write_json(&root.join(ACTIVE_RECEIPT), &receipt)
}

fn existing_live_pass(root: &Path) -> Result<bool, String> {
    let Ok(receipt) = crate::json_boundary::read_json(&root.join(ACTIVE_RECEIPT)) else {
        return Ok(false);
    };
    let store = crate::schema_catalog::load(root);
    Ok(crate::audit::plugin::registry::value_failures(root, &store, &receipt).is_empty())
}

fn fail_closed_agent_types() -> Vec<serde_json::Value> {
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
            "disk_cache_synced": false,
            "global_toml_present": false,
            "exposed": false
        })
    })
    .collect()
}
