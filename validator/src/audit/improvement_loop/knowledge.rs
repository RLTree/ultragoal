use serde_json::Value;
use std::collections::HashSet;
use std::path::Path;

use super::{
    LEARNING_SCHEMA, PROJECTION_REL, PROJECTION_SCHEMA, REGISTRY, REGISTRY_SCHEMA,
    knowledge_projection,
};

pub(super) fn failures(root: &Path) -> Vec<String> {
    let path = root.join(REGISTRY);
    let bytes = match crate::digest::read_file_bytes(&path) {
        Ok(bytes) => bytes,
        Err(err) => {
            return vec![format!(
                "improvement_loop_registry_missing_or_malformed:{err}"
            )];
        }
    };
    let registry = match serde_json::from_slice::<Value>(&bytes) {
        Ok(value) => value,
        Err(err) => {
            return vec![format!(
                "improvement_loop_registry_missing_or_malformed:{err}"
            )];
        }
    };
    let mut out = Vec::new();
    if registry.get("schema").and_then(Value::as_str) != Some(REGISTRY_SCHEMA) {
        out.push("improvement_loop_registry_wrong_schema".to_string());
    }
    let Some(adoption) = registry.get("learning_adoption") else {
        return [
            out,
            vec!["improvement_loop_learning_adoption_missing".to_string()],
        ]
        .concat();
    };
    if adoption.get("schema").and_then(Value::as_str) != Some(LEARNING_SCHEMA) {
        out.push("improvement_loop_learning_adoption_wrong_schema".to_string());
    }
    if adoption.get("one_learning_at_a_time") != Some(&Value::Bool(true)) {
        out.push("improvement_loop_learning_not_one_at_a_time".to_string());
    }
    let Some(records) = adoption.get("records").and_then(Value::as_array) else {
        return [
            out,
            vec!["improvement_loop_learning_records_missing".to_string()],
        ]
        .concat();
    };
    if records.len() < 2 {
        out.push("improvement_loop_learning_records_too_few".to_string());
    }
    let mut record_ids = HashSet::new();
    for row in records {
        let id = text(row, "record_id");
        if id.is_empty() {
            out.push("improvement_loop_learning_record_id_missing".to_string());
        } else if !record_ids.insert(id.clone()) {
            out.push(format!(
                "improvement_loop_learning_record_id_duplicate:{id}"
            ));
        }
    }
    let adopted = records
        .iter()
        .filter(|row| text(row, "status") == "adopted")
        .count();
    let held_or_closed = records
        .iter()
        .filter(|row| {
            matches!(
                text(row, "status").as_str(),
                "held" | "narrowed" | "rejected" | "superseded" | "retired"
            )
        })
        .count();
    if adopted != 1 {
        out.push(format!("improvement_loop_learning_adopted_count:{adopted}"));
    }
    if held_or_closed != 1 {
        out.push(format!(
            "improvement_loop_learning_non_adopted_count:{held_or_closed}"
        ));
    }
    for row in records {
        out.extend(record_failures(row));
    }
    out.extend(projection_failures(root, &bytes, &registry, adoption));
    out
}

fn record_failures(row: &Value) -> Vec<String> {
    let id = text(row, "record_id");
    let prefix = if id.is_empty() {
        "unknown"
    } else {
        id.as_str()
    };
    let status = text(row, "status");
    let decision = text(row, "decision");
    let expected_decision = match status.as_str() {
        "adopted" => Some("adopt"),
        "held" => Some("hold"),
        "narrowed" => Some("narrow"),
        "rejected" => Some("reject"),
        "superseded" => Some("supersede"),
        "retired" => Some("retire"),
        _ => None,
    };
    let mut out = Vec::new();
    if expected_decision != Some(decision.as_str()) {
        out.push(format!(
            "improvement_loop_learning_decision_mismatch:{prefix}"
        ));
    }
    let envelope = text(row, "operating_envelope");
    let budget = row.get("repair_budget");
    if text(budget.unwrap_or(&Value::Null), "task_family").is_empty()
        || text(budget.unwrap_or(&Value::Null), "operating_envelope") != envelope
        || budget
            .and_then(|value| value.get("max_repair_attempts"))
            .and_then(Value::as_u64)
            == Some(0)
        || budget.and_then(|value| value.get("universal_count")) != Some(&Value::Bool(false))
    {
        out.push(format!(
            "improvement_loop_learning_repair_budget_invalid:{prefix}"
        ));
    }
    let evaluation = row.get("evaluation");
    for field in [
        "baseline",
        "visible",
        "held_out",
        "negative_controls",
        "overlap",
        "already_specified",
        "no_change",
        "semantic_mutation",
        "specification_evolution",
        "measures",
    ] {
        if evaluation
            .and_then(|value| value.get(field))
            .and_then(Value::as_array)
            .is_none_or(|items| items.is_empty())
        {
            out.push(format!(
                "improvement_loop_learning_evaluation_missing:{prefix}:{field}"
            ));
        }
    }
    out
}

fn projection_failures(
    root: &Path,
    registry_bytes: &[u8],
    registry: &Value,
    adoption: &Value,
) -> Vec<String> {
    let Some(projection) = adoption.get("knowledge_projection") else {
        return vec!["improvement_loop_knowledge_projection_metadata_missing".to_string()];
    };
    let mut out = Vec::new();
    if text(projection, "path") != PROJECTION_REL
        || text(projection, "schema") != PROJECTION_SCHEMA
        || text(projection, "format") != "markdown"
        || text(projection, "generator") != "validator/src/audit/improvement_loop/mod.rs"
    {
        out.push("improvement_loop_knowledge_projection_metadata_invalid".to_string());
    }
    let projection_path = root.join(PROJECTION_REL);
    let actual = match crate::digest::read_file_bytes(&projection_path) {
        Ok(bytes) => bytes,
        Err(err) => {
            out.push(format!(
                "improvement_loop_knowledge_projection_missing:{err}"
            ));
            return out;
        }
    };
    match knowledge_projection::render(registry_bytes, registry) {
        Ok(expected) if actual == expected.as_bytes() => {}
        Ok(_) => out.push("improvement_loop_knowledge_projection_stale_or_hand_edited".to_string()),
        Err(err) => out.push(format!(
            "improvement_loop_knowledge_projection_unrenderable:{err}"
        )),
    }
    out
}

fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}
