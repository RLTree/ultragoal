use crate::audit::contract::{REQUIRED_AGENTS, REQUIRED_SKILLS};
use serde_json::Value;
use std::collections::BTreeSet;

pub fn fixture_bundle_errors(value: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    for key in [
        "schema",
        "completion_manifest",
        "lane_registry",
        "ready_for_merge",
        "validator_receipt",
        "verification_backlog",
        "amendments",
        "plugin_manifest",
        "automation_tick_receipt",
    ] {
        if value.get(key).is_none() {
            errors.push(format!("{key} is required"));
        }
    }
    errors.extend(completion_manifest_errors(&value["completion_manifest"]));
    errors.extend(lane_registry_errors(&value["lane_registry"]));
    errors.extend(
        super::receipt_schema_rules::validator_receipt_errors(&value["validator_receipt"])
            .into_iter()
            .map(|e| format!("validator_receipt.{e}")),
    );
    errors.extend(backlog_errors(&value["verification_backlog"]));
    errors.extend(plugin_manifest_errors(&value["plugin_manifest"]));
    errors
}

pub fn red_packet_errors(value: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    for key in [
        "schema",
        "id",
        "expected_failure",
        "base_fixture_path",
        "json_patch",
        "materialization",
        "preconditions",
        "postconditions",
    ] {
        if value.get(key).is_none() {
            errors.push(format!("{key} is required"));
        }
    }
    errors
}

pub fn red_catalog_errors(value: &Value) -> Vec<String> {
    let Some(items) = value.as_array() else {
        return vec!["red catalog must be an array".to_string()];
    };
    let ids = items
        .iter()
        .filter_map(|row| row.get("id").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    if ids.len() != items.len() {
        return vec!["red catalog id uniqueness mismatch".to_string()];
    }
    Vec::new()
}

fn completion_manifest_errors(value: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    if value
        .get("required_claim_ids")
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty)
    {
        errors.push("required_claim_ids must be non-empty".to_string());
    }
    for (index, claim) in value
        .get("claims")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
    {
        if claim.get("claim_ceiling_effect").and_then(Value::as_str) == Some("included")
            && claim.get("semantic_classification_receipts").is_none()
        {
            errors.push(format!(
                "claims[{index}].semantic_classification_receipts is required"
            ));
        }
        if claim
            .pointer("/product_cohesion_waiver/reason")
            .and_then(Value::as_str)
            == Some("local_control_surface_only")
        {
            errors.push(format!(
                "claims[{index}].product_cohesion_waiver.reason is not allowed"
            ));
        }
    }
    errors
}

fn lane_registry_errors(value: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let verification_states = &value["root_verification_phases"];
    for key in [
        "pre_merge_lane_gate",
        "post_merge_integration_gate",
        "final_all_lanes_gate",
    ] {
        if verification_states.get(key).is_none() {
            errors.push(format!("root_verification_phases.{key} is required"));
        }
    }
    errors
}

fn backlog_errors(value: &Value) -> Vec<String> {
    value
        .get("rows")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
        .filter(|(_, row)| row.get("attempts").is_none())
        .map(|(index, _)| format!("verification_backlog.rows[{index}].attempts is required"))
        .collect()
}

fn plugin_manifest_errors(value: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let skills = names(value, "skills");
    let agents = names(value, "agents");
    for required in REQUIRED_SKILLS {
        if !skills.contains(*required) {
            errors.push(format!(
                "plugin_manifest.skills missing required {required}"
            ));
        }
    }
    for required in REQUIRED_AGENTS {
        if !agents.contains(*required) {
            errors.push(format!(
                "plugin_manifest.agents missing required {required}"
            ));
        }
    }
    errors
}

fn names<'a>(value: &'a Value, key: &str) -> BTreeSet<&'a str> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| row.get("name").and_then(Value::as_str))
        .collect()
}
