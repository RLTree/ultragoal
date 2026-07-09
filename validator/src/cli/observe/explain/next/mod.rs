use crate::cli::observe::telemetry;
use crate::cli::observe::types::ObserveCommand;
use serde_json::{Value, json};
use std::path::Path;

const INVENTORY_REL: &str = "docs/generated/observability/command-inventory.json";

#[cfg(test)]
mod tests;

pub(super) fn run(root: &Path, command: &ObserveCommand) -> Result<Value, String> {
    let state = crate::cli::current_state::snapshot(root)?;
    plan(root, command, &state)
}

pub(super) fn plan(root: &Path, command: &ObserveCommand, state: &Value) -> Result<Value, String> {
    let inventory =
        crate::json_boundary::read_json(&root.join(INVENTORY_REL)).unwrap_or(Value::Null);
    let candidate = text(&state, "candidate_digest", "missing");
    let blocker = state.get("first_blocker").unwrap_or(&Value::Null);
    let board = state
        .get("observability_control_board")
        .unwrap_or(&Value::Null);
    let first_incomplete = board.get("first_incomplete").unwrap_or(&Value::Null);
    let row_id = text(first_incomplete, "id", text(blocker, "id", "unknown"));
    let family = text(
        first_incomplete,
        "family",
        text(blocker, "surface", "unknown"),
    );
    let row = inventory_row(&inventory, family, row_id);
    let missing_surfaces = missing_surfaces(row, first_incomplete);
    let next_surface = text(
        first_incomplete,
        "next_unobservable_surface",
        text(row, "next_unobservable_surface", "unknown"),
    );
    let narrow_rerun = text(blocker, "narrow_rerun", "ultragoal current-state --json");
    let claim_impact = text(
        row,
        "claim_impact",
        "blocks_observability_product_closure_readiness_release_completion_update_goal",
    );
    let repair = repair_text(row_id, family, next_surface, narrow_rerun);
    let mut receipt = telemetry::base_receipt(root, command, "pass", None)?;
    receipt["schema"] = json!("harness-ultragoal.observe-explain-next-receipt.v1");
    receipt["candidate_digest"] = json!(candidate);
    receipt["target_row"] = json!(row_id);
    receipt["target_family"] = json!(family);
    receipt["owner_surface"] = json!(text(row, "current_owner_surface", family));
    receipt["next_unobservable_surface"] = json!(next_surface);
    receipt["missing_proof_class"] = json!(missing_surfaces.join("; "));
    receipt["claim_impact"] = json!(claim_impact);
    receipt["next_repair"] = json!(repair);
    receipt["narrow_rerun"] = json!(narrow_rerun);
    receipt["required_query_commands"] = json!(query_commands());
    receipt["required_explain_command"] = json!(
        "ultragoal observe explain-failure --run-id <run_id-from-narrow-run> --correlation-id <correlation_id-from-narrow-run>"
    );
    receipt["forbidden_actions"] = json!([
        "do not mark the row observable from this explanation",
        "do not refresh install/cache",
        "do not finalize final packet",
        "do not claim readiness release completion",
        "do not call update_goal",
        "do not launch worktrees"
    ]);
    receipt["explanation_target"] = json!({
        "status": text(first_incomplete, "observability_status", text(board, "status", "blocked")),
        "where_failed": format!("observability_control_board.{family}.{row_id}"),
        "why_failed": format!("{row_id} is missing {next_surface}"),
        "next_repair": repair,
        "candidate_digest": candidate,
        "claim_impact": claim_impact
    });
    receipt["explanation"] = json!({
        "requested_target": "next",
        "fallback_used": false,
        "root_cause": format!("observability control board first incomplete row: {family}/{row_id}"),
        "known_current_failure": [format!("{row_id}:{next_surface}")],
        "evidence_sources": [
            "current-state read model",
            "observability control board",
            INVENTORY_REL
        ],
        "implicated_paths": implicated_paths(row),
        "smallest_repair": receipt["next_repair"],
        "narrow_rerun": narrow_rerun,
        "broad_rerun": "source audit once after narrow observable proof passes if this claim boundary requires broad proof",
        "claim_ceiling": "source-local only; observability product closure readiness release completion final-packet install/cache registry reviewer and update_goal remain blocked",
        "query_evidence": {
            "logs": {"status": "pending_after_narrow_rerun"},
            "metrics": {"status": "pending_after_narrow_rerun"},
            "traces": {"status": "pending_after_narrow_rerun"}
        },
        "repair_guidance": receipt["next_repair"],
        "row": {
            "id": row_id,
            "family": family,
            "owner_surface": text(row, "current_owner_surface", family),
            "observability_status": text(first_incomplete, "observability_status", text(row, "observability_status", "unknown")),
            "next_unobservable_surface": next_surface,
            "missing_surfaces": missing_surfaces,
            "validator_check_id": text(row, "validator_check_id", "unknown"),
            "focused_tests": row.get("focused_tests").cloned().unwrap_or_else(|| json!([])),
            "receipt_paths": row.get("receipt_paths").cloned().unwrap_or_else(|| json!([])),
            "same_candidate_query_proof_paths": row
                .get("same_candidate_query_proof_paths")
                .or_else(|| row.get("live_query_proof_paths"))
                .cloned()
                .unwrap_or_else(|| json!([]))
        }
    });
    receipt["supported_claims"] = json!(["observability_next_repair_plan_only"]);
    receipt["blocked_claims"] = json!([
        "observability_product_closure",
        "readiness",
        "release",
        "completion",
        "final_packet_correctness",
        "install_cache_refresh",
        "worktree_eligibility",
        "update_goal_eligibility"
    ]);
    Ok(receipt)
}

fn inventory_row<'a>(inventory: &'a Value, family: &str, row_id: &str) -> &'a Value {
    let key = match family {
        "commands" => "command_observability_inventory",
        "surfaces" => "surface_inventory",
        "operating_loop" => "operating_loop_inventory",
        "signals" => "signal_inventory",
        "validator_checks" => "validator_check_inventory",
        "receipts" => "receipt_proof_inventory",
        "fixtures" => "fixture_report_inventory",
        "package_setup" => "package_plugin_setup_retrofit_inventory",
        "long_running" => "long_running_path_inventory",
        "external_live" => "external_live_path_inventory",
        "claim_guards" => "claim_guard_inventory",
        _ => "",
    };
    inventory
        .get(key)
        .and_then(|rows| rows.get(row_id))
        .unwrap_or(&Value::Null)
}

fn missing_surfaces(row: &Value, first_incomplete: &Value) -> Vec<String> {
    let mut values = row
        .get("missing_surfaces")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if values.is_empty() {
        let surface = text(first_incomplete, "next_unobservable_surface", "unknown");
        if surface != "unknown" {
            values.push(surface.to_string());
        }
    }
    values
}

fn implicated_paths(row: &Value) -> Value {
    let mut paths = Vec::new();
    for key in [
        "receipt_paths",
        "same_candidate_query_proof_paths",
        "live_query_proof_paths",
    ] {
        if let Some(values) = row.get(key).and_then(Value::as_array) {
            for value in values {
                if let Some(path) = value.as_str() {
                    paths.push(json!(path));
                }
            }
        }
    }
    Value::Array(paths)
}

fn query_commands() -> Vec<&'static str> {
    vec![
        "ultragoal observe logs query --run-id <run_id-from-narrow-run> --correlation-id <correlation_id-from-narrow-run> --limit 100",
        "ultragoal observe metrics query --run-id <run_id-from-narrow-run> --correlation-id <correlation_id-from-narrow-run> --limit 100",
        "ultragoal observe traces query --run-id <run_id-from-narrow-run> --correlation-id <correlation_id-from-narrow-run> --limit 100",
    ]
}

fn repair_text(row_id: &str, family: &str, next_surface: &str, narrow_rerun: &str) -> String {
    format!(
        "repair {next_surface} for {family}/{row_id}, then run `{narrow_rerun}`, query logs metrics traces for the emitted run/correlation ids, and rerun `ultragoal observe explain --next`"
    )
}

fn text<'a>(value: &'a Value, field: &str, default: &'a str) -> &'a str {
    value.get(field).and_then(Value::as_str).unwrap_or(default)
}
