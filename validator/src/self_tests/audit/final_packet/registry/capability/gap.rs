use serde_json::{Value, json};

pub(crate) fn record(raw_digest: &str) -> Value {
    json!({
        "schema":"harness-ultragoal.capability-gap.v1",
        "id":"registry-reviewer-exposure-test",
        "source_artifact":{
            "path":"validation_artifacts/ultragoal-audit/active-registry-observation-current.json",
            "digest":raw_digest
        },
        "source_session_id":"session",
        "observed_at":"2026-06-27T00:00:00Z",
        "affected_workflow":"registry_probe",
        "affected_law_ids":[
            "capability-gap-extraction-harness-capability-promotion",
            "connector-capability-discovery",
            "distribution-sharing-surface-claim-separation"
        ],
        "affected_claim_ids":[
            "app_registry_or_reviewer_exposure",
            "review_readiness",
            "release_readiness",
            "final_packet_correctness",
            "completion",
            "update_goal_eligibility"
        ],
        "missing_capability_class":"live_same_surface_plugin_registry_or_reviewer_exposure",
        "owner_surface":"codex_desktop_plugin_registry",
        "blocked_package_surfaces":["active_registry_exposure","reviewer_exposure"],
        "deterministic_repair_target":"provide_live_tool_registry_query_or_keep_claims_blocked",
        "chosen_promotion_artifact":"validation_artifacts/ultragoal-audit/active-registry-exposure-current.json",
        "current_claim_ceiling":"withheld_or_blocked",
        "required_evidence":["live same-surface tool registry query"],
        "disposition":"open_claim_blocked"
    })
}
