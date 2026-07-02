use serde_json::{Value, json};

const ACTIVE_RECEIPT: &str =
    "validation_artifacts/ultragoal-audit/active-registry-exposure-current.json";
const RAW_OBSERVATION: &str =
    "validation_artifacts/ultragoal-audit/active-registry-observation-current.json";

pub(crate) fn record(
    candidate: &str,
    observed_at: &str,
    session_id: &str,
    raw_digest: &str,
) -> Value {
    json!({
        "schema": "harness-ultragoal.capability-gap.v1",
        "id": format!("registry-reviewer-exposure-{}", candidate_suffix(candidate)),
        "source_artifact": {"path": RAW_OBSERVATION, "digest": raw_digest},
        "source_session_id": session_id,
        "observed_at": observed_at,
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
        "blocked_package_surfaces": [
            "plugins_ui_visibility",
            "marketplace_publication",
            "install_button_success",
            "launcher_runtime_exposure",
            "active_registry_exposure",
            "reviewer_exposure"
        ],
        "deterministic_repair_target": "provide_live_tool_registry_query_or_keep_claims_blocked",
        "chosen_promotion_artifact": ACTIVE_RECEIPT,
        "current_claim_ceiling": "withheld_or_blocked",
        "required_evidence": [
            "live same-surface tool registry query",
            "account/workspace/session boundary",
            "raw observation digest",
            "current reviewer exposure for all required custom agents"
        ],
        "disposition": "open_claim_blocked"
    })
}

fn candidate_suffix(candidate: &str) -> &str {
    candidate
        .strip_prefix("sha256:")
        .unwrap_or(candidate)
        .get(..12)
        .unwrap_or(candidate)
}
