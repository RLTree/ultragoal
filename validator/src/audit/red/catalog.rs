use crate::red::catalog::RedCatalogProjectionRequest;
use crate::schema_catalog::SchemaStore;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const SPECIFIC_GUARD_PREFIX: &str = "mandatory_law_specific_guard_not_enforced:";

pub(crate) fn check(
    root: &Path,
    store: &SchemaStore,
    failures: &mut BTreeMap<String, Vec<String>>,
) {
    let projection = match crate::red::catalog::check(RedCatalogProjectionRequest { root }) {
        Ok(value) => value,
        Err(error) => {
            push(failures, error.stable_text());
            return;
        }
    };
    let ids = projection
        .rows
        .iter()
        .map(|row| row.id.as_str())
        .collect::<BTreeSet<_>>();
    crate::audit::red::identity::check(root, store, &ids, failures);
}

pub(crate) fn law_guard_behavior_verified(root: &Path, id: &str, law: &str, field: &str) -> bool {
    if !semantic_segment(id) || !semantic_segment(law) || !semantic_field(field) {
        return false;
    }
    let expected = format!("{SPECIFIC_GUARD_PREFIX}{law}:{field}");
    let packet_rel = format!("fixtures/red/{id}.json");
    let base_rel = format!("fixtures/mandatory-law-surfaces/valid/{law}.json");
    let Some(packet) = regular_json(root, &packet_rel) else {
        return false;
    };
    let Some(mut materialized) = regular_json(root, &base_rel) else {
        return false;
    };
    if packet.get("schema").and_then(Value::as_str) != Some("harness-ultragoal.red-packet.v1")
        || packet.get("id").and_then(Value::as_str) != Some(id)
        || packet
            .pointer("/expected_failure/error")
            .and_then(Value::as_str)
            != Some(expected.as_str())
        || packet.get("base_fixture_path").and_then(Value::as_str) != Some(base_rel.as_str())
        || packet
            .get("filesystem_fixtures")
            .is_some_and(|value| value.as_array().is_none_or(|rows| !rows.is_empty()))
        || packet.get("review_round_anchor_overrides").is_some()
        || !exact_materialization(&packet)
        || !exact_patch_contract(&packet, field)
        || !exact_precondition(&packet, field)
        || !exact_postcondition(&packet, field, &expected)
        || materialized.get("schema").and_then(Value::as_str)
            != Some("harness-ultragoal.mandatory-law-surface-receipt.v1")
        || materialized.get("law_id").and_then(Value::as_str) != Some(law)
        || materialized
            .pointer(&format!("/law_specific/{field}"))
            .and_then(Value::as_bool)
            != Some(true)
    {
        return false;
    }
    let Some(guards) = materialized
        .get_mut("law_specific")
        .and_then(Value::as_object_mut)
    else {
        return false;
    };
    guards.insert(field.to_string(), Value::Bool(false));
    materialized["red_fixture_ids"] = Value::Array(Vec::new());
    crate::audit::mandatory::law::surfaces::receipt_value_failures_with_candidate(
        root,
        &materialized,
        "",
    )
    .into_iter()
    .filter(|failure| failure.starts_with(SPECIFIC_GUARD_PREFIX))
    .collect::<Vec<_>>()
        == [expected]
}

fn exact_materialization(packet: &Value) -> bool {
    let Some(value) = packet.get("materialization").and_then(Value::as_object) else {
        return false;
    };
    value.len() == 3
        && value
            .get("expected_validation_layer")
            .and_then(Value::as_str)
            == Some("package")
        && value
            .get("first_failure_must_match_expected")
            .and_then(Value::as_bool)
            == Some(true)
        && value
            .get("post_patch_schema_valid")
            .and_then(Value::as_bool)
            == Some(true)
}

fn exact_patch_contract(packet: &Value, field: &str) -> bool {
    let Some(rows) = packet.get("json_patch").and_then(Value::as_array) else {
        return false;
    };
    let Some(row) = rows
        .first()
        .filter(|_| rows.len() == 1)
        .and_then(Value::as_object)
    else {
        return false;
    };
    let expected_path = format!("/law_specific/{field}");
    row.len() == 3
        && row.get("op").and_then(Value::as_str) == Some("replace")
        && row.get("path").and_then(Value::as_str) == Some(expected_path.as_str())
        && row.get("value").and_then(Value::as_bool) == Some(false)
}

fn exact_precondition(packet: &Value, field: &str) -> bool {
    let Some(rows) = packet.get("preconditions").and_then(Value::as_array) else {
        return false;
    };
    let Some(row) = rows
        .first()
        .filter(|_| rows.len() == 1)
        .and_then(Value::as_object)
    else {
        return false;
    };
    let expected_path = format!("/law_specific/{field}");
    row.len() == 2
        && row.get("exists").and_then(Value::as_bool) == Some(true)
        && row.get("path").and_then(Value::as_str) == Some(expected_path.as_str())
}

fn exact_postcondition(packet: &Value, field: &str, expected: &str) -> bool {
    let Some(rows) = packet.get("postconditions").and_then(Value::as_array) else {
        return false;
    };
    let Some(row) = rows
        .first()
        .filter(|_| rows.len() == 1)
        .and_then(Value::as_object)
    else {
        return false;
    };
    let expected_path = format!("/law_specific/{field}");
    row.len() == 2
        && row.get("path").and_then(Value::as_str) == Some(expected_path.as_str())
        && row.get("expectation").and_then(Value::as_str) == Some(expected)
}

fn regular_json(root: &Path, relative: &str) -> Option<Value> {
    let path = root.join(relative);
    let metadata = std::fs::symlink_metadata(&path).ok()?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return None;
    }
    crate::json_boundary::read_json(&path).ok()
}

fn semantic_segment(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 192
        && !value.starts_with('-')
        && !value.ends_with('-')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn semantic_field(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 192
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn push(failures: &mut BTreeMap<String, Vec<String>>, detail: impl Into<String>) {
    failures
        .entry("red-fixture-coverage".to_string())
        .or_default()
        .push(detail.into());
}
