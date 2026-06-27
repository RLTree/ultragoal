use crate::{digest, review::round::ReviewFailure, review::round::anchor::values::AnchorValues};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

pub(crate) fn persona_errors(
    root: &Path,
    value: &Value,
    anchors: &AnchorValues,
    out: &mut Vec<ReviewFailure>,
) {
    let rows = value
        .get("reviewers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let seen_personas = rows
        .iter()
        .filter_map(|row| row.get("persona").and_then(Value::as_str))
        .map(str::to_string)
        .collect::<BTreeSet<_>>();
    required_persona_errors(&rows, &seen_personas, out);
    crate::review::round::registry::exposure_errors(root, value, out);
    prior_reviewer_errors(value, &rows, out);
    let mut row_seen_personas = BTreeSet::new();
    let mut seen_agents = BTreeSet::new();
    for row in &rows {
        row_errors(
            root,
            value,
            row,
            anchors,
            &mut row_seen_personas,
            &mut seen_agents,
            out,
        );
    }
}

fn required_persona_errors(
    rows: &[Value],
    seen_personas: &BTreeSet<String>,
    out: &mut Vec<ReviewFailure>,
) {
    let required = crate::review::round::config::PERSONAS
        .iter()
        .map(|spec| spec.persona.to_string())
        .collect::<BTreeSet<_>>();
    if rows.len() != crate::review::round::config::PERSONAS.len() {
        out.push(ReviewFailure::new(
            "validator-execution-provenance",
            "review_round_wrong_persona_count",
            rows.len().to_string(),
        ));
    }
    for missing in required.difference(seen_personas) {
        out.push(ReviewFailure::new(
            "validator-execution-provenance",
            "review_round_missing_persona",
            missing.clone(),
        ));
    }
}

fn prior_reviewer_errors(value: &Value, rows: &[Value], out: &mut Vec<ReviewFailure>) {
    let Some(prior_rows) = value
        .get("prior_round_reviewer_agent_ids")
        .and_then(Value::as_array)
    else {
        out.push(ReviewFailure::new(
            "validator-execution-provenance",
            "review_round_prior_reviewer_ids_missing",
            "prior_round_reviewer_agent_ids",
        ));
        return;
    };
    let prior = prior_rows
        .iter()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    for row in rows {
        let agent = row
            .get("reviewer_agent_id")
            .and_then(Value::as_str)
            .unwrap_or("");
        if prior.contains(agent) {
            out.push(ReviewFailure::new(
                "validator-execution-provenance",
                "review_round_reused_reviewer",
                agent,
            ));
        }
    }
}

fn row_errors(
    root: &Path,
    receipt: &Value,
    row: &Value,
    anchors: &AnchorValues,
    seen_personas: &mut BTreeSet<String>,
    seen_agents: &mut BTreeSet<String>,
    out: &mut Vec<ReviewFailure>,
) {
    let persona = row.get("persona").and_then(Value::as_str).unwrap_or("");
    if !seen_personas.insert(persona.to_string()) {
        out.push(ReviewFailure::new(
            "validator-execution-provenance",
            "review_round_duplicate_persona",
            persona,
        ));
    }
    let agent = row
        .get("reviewer_agent_id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if !seen_agents.insert(agent) {
        out.push(ReviewFailure::new(
            "validator-execution-provenance",
            "review_round_reused_reviewer",
            persona,
        ));
    }
    let Some(spec) = crate::review::round::config::persona_spec(persona) else {
        return;
    };
    crate::review::round::registry::row_agent_type_error(row, persona, out);
    crate::review::round::spawn::receipts::spawn_receipt_errors(receipt, row, persona, out);
    prompt_identity_errors(root, row, persona, spec.prompt_path, out);
    custom_agent_identity_errors(root, row, persona, spec.custom_path, out);
    crate::review::round::row::policy::row_policy_errors(row, receipt, anchors, persona, out);
    crate::review::round::product::fitness::disposition_errors(root, row, receipt, persona, out);
    crate::review::round::claim::ceiling::row_authority_errors(
        root, receipt, anchors, row, persona, out,
    );
    crate::review::round::artifacts::artifact_binding_errors(root, row, persona, out);
    crate::review::round::report::report_errors(root, receipt, row, anchors, persona, out);
}

fn prompt_identity_errors(
    root: &Path,
    row: &Value,
    persona: &str,
    path: &str,
    out: &mut Vec<ReviewFailure>,
) {
    let digest = digest::file(&root.join(path)).unwrap_or_else(|_| String::new());
    if row.get("persona_prompt_path").and_then(Value::as_str) != Some(path)
        || digest.is_empty()
        || digest == crate::digest::ZERO
        || row.get("persona_prompt_digest").and_then(Value::as_str) != Some(&digest)
    {
        out.push(ReviewFailure::new(
            "validator-execution-provenance",
            "review_round_substituted_persona_prompt",
            persona,
        ));
    }
}

fn custom_agent_identity_errors(
    root: &Path,
    row: &Value,
    persona: &str,
    path: &str,
    out: &mut Vec<ReviewFailure>,
) {
    let digest = digest::file(&root.join(path)).unwrap_or_else(|_| String::new());
    if row.get("custom_agent_path").and_then(Value::as_str) != Some(path)
        || digest.is_empty()
        || digest == crate::digest::ZERO
        || row.get("custom_agent_digest").and_then(Value::as_str) != Some(&digest)
    {
        out.push(ReviewFailure::new(
            "validator-execution-provenance",
            "review_round_custom_agent_mismatch",
            persona,
        ));
    }
}
