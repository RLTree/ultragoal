use crate::{review::round::ReviewFailure, review::round::anchor::values::AnchorValues};
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
    let seen_roles = rows
        .iter()
        .filter_map(|row| row.get("role").and_then(Value::as_str))
        .map(str::to_string)
        .collect::<BTreeSet<_>>();
    required_role_errors(&rows, &seen_roles, out);
    let registry = crate::review::round::registry::exposure_errors(root, value, out);
    prior_reviewer_errors(value, &rows, out);
    let mut row_seen_roles = BTreeSet::new();
    let mut seen_agents = BTreeSet::new();
    for row in &rows {
        row_errors(
            root,
            value,
            row,
            anchors,
            &mut row_seen_roles,
            &mut seen_agents,
            &registry,
            out,
        );
    }
}

fn required_role_errors(
    rows: &[Value],
    seen_roles: &BTreeSet<String>,
    out: &mut Vec<ReviewFailure>,
) {
    let required = crate::review::round::config::REVIEW_ROLES
        .iter()
        .map(|spec| spec.role_name.to_string())
        .collect::<BTreeSet<_>>();
    if rows.len() != crate::review::round::config::REVIEW_ROLES.len() {
        out.push(ReviewFailure::new(
            "validator-execution-provenance",
            "review_round_wrong_role_count",
            rows.len().to_string(),
        ));
    }
    for missing in required.difference(seen_roles) {
        out.push(ReviewFailure::new(
            "validator-execution-provenance",
            "review_round_missing_role",
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
                "prior-reviewer-id",
            ));
        }
    }
}

fn row_errors(
    root: &Path,
    receipt: &Value,
    row: &Value,
    anchors: &AnchorValues,
    seen_roles: &mut BTreeSet<String>,
    seen_agents: &mut BTreeSet<String>,
    registry: &crate::review::round::registry::RegistrySnapshot,
    out: &mut Vec<ReviewFailure>,
) {
    let role = row.get("role").and_then(Value::as_str).unwrap_or("");
    if !seen_roles.insert(role.to_string()) {
        out.push(ReviewFailure::new(
            "validator-execution-provenance",
            "review_round_duplicate_role",
            "reviewer-role",
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
            "reviewer-agent-id",
        ));
    }
    let Some(spec) = crate::review::round::config::review_role_spec(role) else {
        return;
    };
    crate::review::round::registry::row_agent_role_error(row, role, out);
    crate::review::round::spawn::receipts::spawn_receipt_errors(receipt, row, role, out);
    agent_manifest_identity_errors(row, role, spec, registry, out);
    crate::review::round::row::policy::row_policy_errors(row, receipt, anchors, role, out);
    crate::review::round::product::fitness::disposition_errors(root, row, receipt, role, out);
    crate::review::round::claim::ceiling::row_authority_errors(
        root, receipt, anchors, row, role, out,
    );
    crate::review::round::artifacts::artifact_binding_errors(root, row, role, out);
    crate::review::round::report::report_errors(root, receipt, row, anchors, role, out);
}

fn agent_manifest_identity_errors(
    row: &Value,
    role: &str,
    spec: &crate::review::round::config::ReviewRoleSpec,
    registry: &crate::review::round::registry::RegistrySnapshot,
    out: &mut Vec<ReviewFailure>,
) {
    let path = crate::review::round::config::agent_role(spec).manifest_path;
    let digest = registry.manifest_digest(role, path);
    if row.get("agent_manifest_path").and_then(Value::as_str) != Some(path)
        || digest.is_none_or(|digest| digest == crate::digest::ZERO)
        || row.get("agent_manifest_digest").and_then(Value::as_str) != digest
    {
        out.push(ReviewFailure::new(
            "validator-execution-provenance",
            "review_round_agent_manifest_mismatch",
            role,
        ));
    }
}
