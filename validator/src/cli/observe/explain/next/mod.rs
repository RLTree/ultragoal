use crate::cli::observe::command::ObserveCommand;
use serde_json::{Value, json};
use std::path::Path;

#[cfg(test)]
mod tests;

const BLOCKER: &str = "HCT-OBSERVE successor catalog unavailable/not adopted";

pub(super) fn run(root: &Path, command: &ObserveCommand) -> Result<Value, String> {
    let state = crate::cli::current_state::snapshot(root)?;
    plan(root, command, &state)
}

pub(super) fn plan(
    _root: &Path,
    _command: &ObserveCommand,
    state: &Value,
) -> Result<Value, String> {
    let candidate = text(state, "candidate_digest", "missing");
    let repair = "implement and adopt the typed candidate-bound HCT-OBSERVE successor catalog";
    let narrow_rerun = "ultragoal current-state --json";
    let explanation_target = json!({
        "status": "unavailable",
        "where_failed": "HCT-OBSERVE adoption boundary",
        "why_failed": BLOCKER,
        "next_repair": repair,
        "candidate_digest": candidate,
        "claim_impact": "blocks_observability_product_closure_readiness_release_completion_update_goal"
    });
    let row = json!({
        "id": "HCT-OBSERVE",
        "family": "successor_catalog",
        "owner_surface": "OWN-OBSERVABILITY",
        "observability_status": "unavailable",
        "next_unobservable_surface": "typed candidate-bound successor catalog",
        "missing_surfaces": ["accepted HCT-OBSERVE catalog output"],
        "validator_check_id": "not_adopted",
        "focused_tests": [],
        "receipt_paths": [],
        "same_candidate_query_proof_paths": []
    });
    let explanation = json!({
        "requested_target": "next",
        "fallback_used": false,
        "root_cause": BLOCKER,
        "known_current_failure": [BLOCKER],
        "evidence_sources": ["current-state capability projection"],
        "implicated_paths": [],
        "smallest_repair": repair,
        "narrow_rerun": narrow_rerun,
        "broad_rerun": "source audit only after HCT-OBSERVE adoption and narrow verification",
        "claim_ceiling": "unavailable dependency; observability readiness release completion and update_goal remain blocked",
        "query_evidence": {
            "logs": {"status": "unavailable"},
            "metrics": {"status": "unavailable"},
            "traces": {"status": "unavailable"}
        },
        "repair_guidance": repair,
        "row": row
    });
    let mut receipt = json!({
        "schema": "harness-ultragoal.observe-explain-next-receipt.v1",
        "status": "fail",
        "candidate_digest": candidate,
        "failure_class": "hct_observe_successor_catalog_unavailable",
        "why_failed": BLOCKER,
        "where_failed": "HCT-OBSERVE adoption boundary",
        "target_row": "HCT-OBSERVE",
        "target_family": "successor_catalog",
        "owner_surface": "OWN-OBSERVABILITY",
        "next_unobservable_surface": "typed candidate-bound successor catalog",
        "missing_proof_class": "accepted HCT-OBSERVE catalog output",
        "claim_impact": "blocks_observability_product_closure_readiness_release_completion_update_goal",
        "next_repair": repair,
        "narrow_rerun": narrow_rerun,
        "required_query_commands": [],
        "explanation_target": explanation_target,
        "explanation": explanation
    });
    receipt.as_object_mut().expect("receipt object").extend(
        json!({
            "forbidden_actions": [
                "do not infer observability from retained command-inventory bytes",
                "do not mark any row observable",
                "do not refresh install/cache",
                "do not finalize final packet",
                "do not claim readiness release completion",
                "do not call update_goal"
            ],
            "supported_claims": [],
            "blocked_claims": [
                "observability_product_closure",
                "readiness",
                "release",
                "completion",
                "final_packet_correctness",
                "install_cache_refresh",
                "worktree_eligibility",
                "update_goal_eligibility"
            ],
            "claim_ceiling": "withheld_or_blocked"
        })
        .as_object()
        .expect("receipt fields")
        .clone(),
    );
    Ok(receipt)
}

fn text<'a>(value: &'a Value, field: &str, default: &'a str) -> &'a str {
    value.get(field).and_then(Value::as_str).unwrap_or(default)
}
