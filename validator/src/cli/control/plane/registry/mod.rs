use crate::cli::control::plane::operation::ControlOperation;
use serde_json::json;
use std::path::{Path, PathBuf};

mod agent_rows;
mod capability;
pub(crate) mod stdout;
pub(crate) mod telemetry;

pub(crate) const ACTIVE_RECEIPT: &str =
    "validation_artifacts/ultragoal-audit/active-registry-exposure-current.json";
const RAW_OBSERVATION: &str =
    "validation_artifacts/ultragoal-audit/active-registry-observation-current.json";

pub(crate) fn mint_fail_closed_if_needed(
    root: &Path,
    operation: ControlOperation,
    candidate: &str,
    authority_roots: Option<&super::AgentAuthorityRoots>,
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
    let boundary = runtime_boundary();
    let account_id = boundary.account_id;
    let workspace_id = boundary.workspace_id;
    let session_id = boundary.session_id;
    let call_id = fail_closed_call_id(operation, candidate);
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
            "final_packet_correctness",
            "update_goal_eligibility"
        ]
    });
    let raw_path = crate::output_path::literal_claim_artifact_path(
        root,
        RAW_OBSERVATION,
        "registry raw observation",
    );
    crate::json_boundary::write_json(&raw_path, &raw)?;
    let raw_digest = crate::digest::file(&raw_path)?;
    let receipt = json!({
        "schema": "harness-ultragoal.multi-agent-registry-exposure.v1",
        "generated_at": now,
        "captured_at": now,
        "status": "fail",
        "issuer": {"tool": "ultragoal", "authority": "cli_control_plane"},
        "tool_call": {
            "name": "ultragoal registry probe",
            "call_id": call_id,
            "arguments_digest": crate::digest::ZERO
        },
        "capture_method": "fail_closed_no_capability",
        "boundary": {
            "account_id": account_id,
            "workspace_id": workspace_id,
            "session_id": session_id.clone()
        },
        "source": "ultragoal.registry_probe",
        "target_revision": {"kind": "package_digest", "value": candidate},
        "claim_ceiling": "withheld_or_blocked",
        "session_id": session_id,
        "round_id": fail_closed_round_id(operation, candidate),
        "raw_observation": {"path": RAW_OBSERVATION, "digest": raw_digest},
        "capability_gap": capability::gap::record(candidate, &now, &session_id, &raw_digest),
        "agent_types": agent_types_with_local_state(
            root,
            candidate,
            &session_id,
            authority_roots,
        ),
        "failure": {
            "reason": "live_registry_reviewer_exposure_not_proven",
            "observed": "same-surface registry/reviewer proof unavailable; disk source/install/cache proof is not accepted as a substitute",
            "blocked_claim_classes": [
                "app_registry_or_reviewer_exposure",
                "review_readiness",
                "release_readiness",
                "final_packet_correctness",
                "completion",
                "update_goal_eligibility"
            ]
        }
    });
    let active_path = crate::output_path::literal_claim_artifact_path(
        root,
        ACTIVE_RECEIPT,
        "registry exposure receipt",
    );
    crate::json_boundary::write_json(&active_path, &receipt)
}

#[derive(Clone)]
pub(crate) struct RegistryBoundary {
    pub(crate) account_id: String,
    pub(crate) workspace_id: String,
    pub(crate) session_id: String,
}

pub(crate) fn boundary_from_values(
    account_id: Option<&str>,
    workspace_id: Option<&str>,
    session_id: Option<&str>,
) -> RegistryBoundary {
    RegistryBoundary {
        account_id: normalized_or_unavailable(account_id),
        workspace_id: normalized_or_unavailable(workspace_id),
        session_id: normalized_or_unavailable(session_id),
    }
}

fn runtime_boundary() -> RegistryBoundary {
    boundary_from_values(
        std::env::var("CODEX_ACCOUNT_ID").ok().as_deref(),
        std::env::var("CODEX_WORKSPACE_ID").ok().as_deref(),
        std::env::var("CODEX_THREAD_ID").ok().as_deref(),
    )
}

fn normalized_or_unavailable(value: Option<&str>) -> String {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return "unavailable".to_string();
    };
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':' | '/'))
    {
        value.to_string()
    } else {
        "unavailable".to_string()
    }
}

fn fail_closed_call_id(operation: ControlOperation, candidate: &str) -> String {
    format!(
        "{}-fail-closed-{}",
        operation.id(),
        candidate_suffix(candidate)
    )
}

fn fail_closed_round_id(operation: ControlOperation, candidate: &str) -> String {
    format!(
        "{}-unsupported-live-surface-{}",
        operation.id(),
        candidate_suffix(candidate)
    )
}

fn candidate_suffix(candidate: &str) -> &str {
    candidate
        .strip_prefix("sha256:")
        .unwrap_or(candidate)
        .get(..12)
        .unwrap_or(candidate)
}

fn existing_live_pass(root: &Path) -> Result<bool, String> {
    let receipt_path = crate::output_path::literal_claim_artifact_path(
        root,
        ACTIVE_RECEIPT,
        "active registry receipt",
    );
    let Ok(receipt) = crate::json_boundary::read_json(&receipt_path) else {
        return Ok(false);
    };
    let store = crate::schema_catalog::load(root);
    Ok(crate::audit::plugin::registry::value_failures(root, &store, &receipt).is_empty())
}

fn agent_types_with_local_state(
    root: &Path,
    candidate: &str,
    session_id: &str,
    authority_roots: Option<&super::AgentAuthorityRoots>,
) -> Vec<serde_json::Value> {
    agent_rows::for_home(
        root,
        authority_roots
            .map(|roots| roots.home.clone())
            .or_else(|| std::env::var_os("HOME").map(PathBuf::from)),
        candidate,
        session_id,
        authority_roots.map(|roots| roots.package.clone()),
        authority_roots.map(|roots| roots.project.clone()),
    )
}

#[cfg(test)]
pub(crate) fn agent_types_for_home(
    root: &Path,
    home: Option<PathBuf>,
    candidate: &str,
    session_id: &str,
    package_root: Option<PathBuf>,
    project_root: Option<PathBuf>,
) -> Vec<serde_json::Value> {
    agent_rows::for_home(
        root,
        home,
        candidate,
        session_id,
        package_root,
        project_root,
    )
}
