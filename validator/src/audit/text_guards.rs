mod moving_values;
mod stale_review;

use crate::json_boundary;
use serde_json::Value;
use std::path::Path;

pub fn stale_review_law_failures(root: &Path) -> Vec<String> {
    stale_review::failures(root)
}

pub fn stale_review_law_value_failures(value: &Value) -> Vec<String> {
    stale_review::value_failures(value)
}

pub fn moving_value_drift_failures(root: &Path) -> Vec<String> {
    moving_values::failures(root)
}

pub fn moving_value_value_failures(value: &Value) -> Vec<String> {
    moving_values::value_failures(value)
}

pub fn private_path_value_failures(value: &Value) -> Vec<String> {
    let text = serde_json::to_string(value).unwrap_or_default();
    if text.contains(private_home_marker()) || text.contains(private_tmp_marker()) {
        vec!["manifest_owned_private_local_path".to_string()]
    } else {
        Vec::new()
    }
}

pub fn source_card_freshness_failures(root: &Path) -> Vec<String> {
    match json_boundary::read_json(&root.join("docs/source-cards.json")) {
        Ok(value) => source_card_value_failures(&value),
        Err(err) => vec![format!("docs/source-cards.json: {err}")],
    }
}

pub fn source_card_value_failures(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(source_card_failure)
        .collect()
}

fn source_card_failure(card: &Value) -> Option<String> {
    if card
        .pointer("/retrieval_receipt/status")
        .and_then(Value::as_str)
        != Some("not_refreshed")
    {
        return None;
    }
    let text = format!(
        "{} {}",
        card.get("notes").and_then(Value::as_str).unwrap_or(""),
        card.get("cited_claims")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|claim| claim.get("source_support").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join(" ")
    )
    .to_ascii_lowercase();
    let claims_current = [
        "current implementation",
        "live implementation",
        "source-backed",
    ]
    .iter()
    .any(|phrase| text.contains(phrase));
    let ceiling_lowered = text.contains("verify exact") || text.contains("claim ceiling");
    if claims_current && !ceiling_lowered {
        Some(format!(
            "{}: source_card_not_refreshed_overclaim",
            card.get("id").and_then(Value::as_str).unwrap_or("unknown")
        ))
    } else {
        None
    }
}

pub fn private_home_path_failures(root: &Path) -> Vec<String> {
    let Ok(manifest) = json_boundary::read_json(&root.join("plugin-manifest-draft.json")) else {
        return Vec::new();
    };
    crate::package::inventory::inventory_paths(&manifest)
        .into_iter()
        .filter(|rel| !private_path_fixture_example(rel))
        .filter_map(|rel| private_path_failure(root, &rel))
        .collect()
}

fn private_path_fixture_example(rel: &str) -> bool {
    rel.starts_with("fixtures/red/") || rel.starts_with("fixtures/target-repo/red/")
}

fn private_path_failure(root: &Path, rel: &str) -> Option<String> {
    let path = crate::package::inventory::resolve(root, rel).ok()?;
    let bytes = crate::digest::read_file_bytes(&path).ok()?;
    let text = String::from_utf8(bytes).ok()?;
    if text.contains(private_home_marker()) || text.contains(private_tmp_marker()) {
        Some(format!("{rel}: manifest_owned_private_local_path"))
    } else {
        None
    }
}

fn private_home_marker() -> &'static str {
    concat!("/", "Users/")
}

fn private_tmp_marker() -> &'static str {
    concat!("/", "private", "/tmp/")
}
