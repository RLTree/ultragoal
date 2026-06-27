use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

mod dependencies;
mod production;
mod registry;

const REGISTRY: &str = "docs/mandatory-law-surfaces.json";
const REQUIRED_LAWS: &[&str] = crate::audit::mandatory::law::surface::ids::REQUIRED_LAWS;
const WEAK_TERMS: &[&str] = &[
    "partial",
    "backlog",
    "blocked",
    "future",
    "follow-up",
    "reviewer-only",
    "documentation-only",
    "prose-only",
    "row-shape-only",
    "claim-ceiling-only",
    "stale-source-backed",
];

pub fn package_failures(root: &Path) -> Vec<String> {
    let registry = match crate::json_boundary::read_json(&root.join(REGISTRY)) {
        Ok(value) => value,
        Err(err) => return vec![format!("{REGISTRY}: {err}")],
    };
    let mut out = value_failures(root, &registry);
    let store = crate::schema_catalog::load(root);
    for receipt in registry
        .get("laws")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        out.extend(receipt_value_failures(root, receipt));
        if let Some(law) = receipt.get("law_id").and_then(Value::as_str) {
            out.extend(dependencies::anti_theater_failures(root, &store, law));
        }
    }
    out
}

pub fn package_failures_with_current(
    root: &Path,
    current_failures: &BTreeMap<String, Vec<String>>,
) -> Vec<String> {
    let registry = match crate::json_boundary::read_json(&root.join(REGISTRY)) {
        Ok(value) => value,
        Err(err) => return vec![format!("{REGISTRY}: {err}")],
    };
    let mut out = package_failures(root);
    for receipt in registry
        .get("laws")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let law = receipt
            .get("law_id")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        out.extend(production::current_check_failures(
            receipt,
            law,
            current_failures,
        ));
    }
    out
}

#[cfg(test)]
pub(crate) fn current_check_failures_for_test(
    value: &Value,
    law: &str,
    failures: &BTreeMap<String, Vec<String>>,
) -> Vec<String> {
    production::current_check_failures(value, law, failures)
}

#[cfg(test)]
pub(crate) fn anti_theater_dependency_failures_for_test(
    root: &Path,
    store: &crate::schema_catalog::SchemaStore,
    law: &str,
) -> Vec<String> {
    dependencies::anti_theater_failures(root, store, law)
}

pub fn value_failures(root: &Path, value: &Value) -> Vec<String> {
    let Some(rows) = value.get("laws").and_then(Value::as_array) else {
        return vec!["mandatory_law_registry_missing_laws".to_string()];
    };
    let mut out = Vec::new();
    let seen = rows
        .iter()
        .filter_map(|row| row.get("law_id").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    for law in REQUIRED_LAWS {
        if !seen.contains(law) {
            out.push(format!("mandatory_law_missing:{law}"));
        }
    }
    let red_ids = registry::red_fixture_ids(root);
    for row in rows {
        let law = row
            .get("law_id")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        if contains_weak_term(row) {
            out.push(format!("mandatory_law_weak_disposition:{law}"));
        }
        if !registry::standards_row_exists(root, law) {
            out.push(format!("mandatory_law_missing_standards_row:{law}"));
        }
        if !registry::source_obligation_exists(root, law) {
            out.push(format!("mandatory_law_missing_source_obligation:{law}"));
        }
        if !registry::trace_entry_exists(root, law) {
            out.push(format!("mandatory_law_missing_foundational_trace:{law}"));
        }
        if let Some(path) = row.get("valid_fixture_path").and_then(Value::as_str) {
            if !root.join(path).is_file() {
                out.push(format!("mandatory_law_missing_valid_fixture:{law}"));
            }
        }
        for red in row
            .get("red_fixture_ids")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            if !red_ids.contains(red) {
                out.push(format!("mandatory_law_missing_red_fixture:{law}:{red}"));
            }
        }
    }
    out
}

pub fn receipt_value_failures(root: &Path, value: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let law = value
        .get("law_id")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    if value.get("schema").and_then(Value::as_str)
        != Some("harness-ultragoal.mandatory-law-surface-receipt.v1")
    {
        out.push(format!("mandatory_law_wrong_schema:{law}"));
    }
    if value.get("enforcement_status").and_then(Value::as_str) != Some("deterministic_fail_closed")
    {
        out.push(format!("mandatory_law_not_fail_closed:{law}"));
    }
    for key in [
        "standards_row_id",
        "source_obligation_id",
        "foundational_trace_id",
        "validator_check_id",
        "valid_fixture_path",
        "claim_ceiling_guard",
    ] {
        if value
            .get(key)
            .and_then(Value::as_str)
            .is_none_or(|text| text.trim().is_empty())
        {
            out.push(format!("mandatory_law_missing_field:{law}:{key}"));
        }
    }
    if value
        .get("red_fixture_ids")
        .and_then(Value::as_array)
        .is_none_or(Vec::is_empty)
    {
        out.push(format!("mandatory_law_missing_red_fixtures:{law}"));
    }
    if value
        .get("behavior_failure_modes")
        .and_then(Value::as_array)
        .is_none_or(Vec::is_empty)
    {
        out.push(format!("mandatory_law_missing_failure_modes:{law}"));
    }
    let Some(fields) = value.get("law_specific").and_then(Value::as_object) else {
        out.push(format!("mandatory_law_missing_specific_guards:{law}"));
        return out;
    };
    for (field, enabled) in fields {
        if enabled.as_bool() != Some(true) {
            out.push(format!(
                "mandatory_law_specific_guard_not_enforced:{law}:{field}"
            ));
        }
    }
    for artifact in value
        .get("evidence_artifacts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(path) = artifact.get("path").and_then(Value::as_str) else {
            out.push(format!("mandatory_law_evidence_digest_mismatch:{law}"));
            continue;
        };
        let Some(expected) = artifact.get("digest").and_then(Value::as_str) else {
            out.push(format!("mandatory_law_evidence_digest_mismatch:{law}"));
            continue;
        };
        match crate::digest::file(&root.join(path)) {
            Ok(actual) if actual == expected => {}
            _ => out.push(format!("mandatory_law_evidence_digest_mismatch:{law}")),
        }
    }
    out.extend(production::binding_failures(value, law));
    out
}

fn contains_weak_term(value: &Value) -> bool {
    let text = [
        "claim_ceiling_guard",
        "enforcement_status",
        "validator_check_id",
        "standards_row_id",
        "source_obligation_id",
    ]
    .iter()
    .filter_map(|key| value.get(*key).and_then(Value::as_str))
    .collect::<Vec<_>>()
    .join(" ")
    .to_ascii_lowercase();
    WEAK_TERMS.iter().any(|term| text.contains(term))
}
