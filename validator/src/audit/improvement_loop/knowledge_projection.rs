use serde_json::Value;

use super::PROJECTION_SCHEMA;

pub(super) fn render(registry_bytes: &[u8], registry: &Value) -> Result<String, String> {
    let adoption = registry
        .get("learning_adoption")
        .ok_or_else(|| "learning adoption missing".to_string())?;
    let records = adoption
        .get("records")
        .and_then(Value::as_array)
        .ok_or_else(|| "learning records missing".to_string())?;
    let digest = crate::digest::bytes(registry_bytes);
    let mut output = "# Improvement Loop Knowledge Projection\n\nGenerated from ".to_string();
    output.push_str(
        "`docs/improvement-loop-registry.json`; this file is a read-only projection.\n\n",
    );
    output.push_str(&format!(
        "- Projection schema: `{PROJECTION_SCHEMA}`\n- Registry digest: `{digest}`\n- Generator: `validator/src/audit/improvement_loop/mod.rs`\n\n"
    ));
    for (index, row) in records.iter().enumerate() {
        render_record(&mut output, row);
        if index + 1 < records.len() {
            output.push('\n');
        }
    }
    Ok(output)
}

fn render_record(output: &mut String, row: &Value) {
    output.push_str(&format!("## {}\n\n", one_line(&text(row, "record_id"))));
    output.push_str(&format!(
        "- Status: `{}`\n- Decision: `{}`\n- Owner: {}\n- Version: {}\n- Review date: {}\n- Operating envelope: {}\n- Responsible layer: `{}`\n\n",
        one_line(&text(row, "status")),
        one_line(&text(row, "decision")),
        one_line(&text(row, "owner")),
        one_line(&text(row, "version")),
        one_line(&text(row, "review_date")),
        one_line(&text(row, "operating_envelope")),
        one_line(&text(row, "responsible_layer"))
    ));
    for (label, field) in [
        ("Observation", "observation"),
        ("Mechanism", "mechanism_hypothesis"),
        ("Intervention", "proposed_intervention"),
    ] {
        output.push_str(&format!("**{label}.** {}\n\n", one_line(&text(row, field))));
    }
    output.push_str(&format!("**Evidence.** {}\n\n", list(row, "results")));
    output.push_str(&format!(
        "**Counterevidence.** {}\n\n",
        list(row, "counterevidence")
    ));
    render_evaluation(output, row.get("evaluation"));
    render_budget(output, row.get("repair_budget"));
    output.push_str(&format!(
        "**Rollback.** {}\n\n",
        one_line(&text(row, "rollback"))
    ));
    output.push_str(&format!(
        "**Expiry or invalidation.** {}\n\n",
        list(row, "expiry_or_invalidation")
    ));
    output.push_str(&format!(
        "**Authority handoff.** {}\n",
        one_line(&text(row, "authority_handoff"))
    ));
}

fn render_evaluation(output: &mut String, evaluation: Option<&Value>) {
    let Some(evaluation) = evaluation else {
        return;
    };
    output.push_str("### Evaluation\n\n");
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
        output.push_str(&format!(
            "- {}: {}\n",
            field.replace('_', " "),
            list(evaluation, field)
        ));
    }
    output.push('\n');
}

fn render_budget(output: &mut String, budget: Option<&Value>) {
    let Some(budget) = budget else {
        return;
    };
    output.push_str(&format!(
        "**Repair budget.** {} attempts for `{}` in {}. {}\n\n",
        budget
            .get("max_repair_attempts")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        one_line(&text(budget, "task_family")),
        one_line(&text(budget, "operating_envelope")),
        one_line(&text(budget, "calibration_basis"))
    ));
}

fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn list(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(one_line)
                .collect::<Vec<_>>()
                .join("; ")
        })
        .unwrap_or_default()
}

fn one_line(value: &str) -> String {
    value.replace(['\r', '\n'], " ").replace('`', "'")
}
