use crate::claim_semantics;
use crate::json_boundary;
use crate::red::fixture::row::{expected_check, expected_status, invalid_row, result_row};
use crate::schema_catalog::SchemaStore;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

pub fn red_fixture_results(
    root: &Path,
    store: &SchemaStore,
    validator_digests: &BTreeMap<String, String>,
) -> BTreeMap<String, Value> {
    let Ok(rows) = json_boundary::read_json(&root.join("templates/RED_FIXTURES.json")) else {
        return BTreeMap::new();
    };
    let Some(items) = rows.as_array() else {
        return BTreeMap::new();
    };
    let mut results = BTreeMap::new();
    for row in items {
        let row_id = row
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("invalid-row");
        let result = result_for_row(root, store, validator_digests, row);
        results.insert(row_id.to_string(), result);
    }
    results
}

fn result_for_row(
    root: &Path,
    store: &SchemaStore,
    validator_digests: &BTreeMap<String, String>,
    row: &Value,
) -> Value {
    let packet_rel = row.get("packet_path").and_then(Value::as_str).unwrap_or("");
    let row_expected = row.get("expected_failure").unwrap_or(&Value::Null);
    if crate::package::inventory::package_path_error(root, packet_rel).is_some() {
        return invalid_row(row, "red_fixture_packet_path_invalid");
    }
    let packet_path = match crate::package::inventory::resolve(root, packet_rel) {
        Ok(path) if path.is_file() => path,
        _ => {
            return result_row(
                root,
                packet_rel,
                row_expected,
                "red_fixture_packet_missing",
                "fail",
                row.get("packet_digest").and_then(Value::as_str),
                None,
            );
        }
    };
    let packet = match json_boundary::read_json(&packet_path) {
        Ok(value) if value.is_object() => value,
        _ => {
            return result_row(
                root,
                packet_rel,
                row_expected,
                "red_fixture_packet_malformed_json",
                "fail",
                None,
                None,
            );
        }
    };
    materialized_result(root, store, validator_digests, &packet, packet_rel)
}

fn materialized_result(
    root: &Path,
    store: &SchemaStore,
    validator_digests: &BTreeMap<String, String>,
    packet: &Value,
    packet_rel: &str,
) -> Value {
    let expected = &packet["expected_failure"];
    let _filesystem_guard = match crate::red::filesystem::fixtures::materialize(root, packet) {
        Ok(guard) => guard,
        Err(error) => return simple_row(root, packet_rel, expected, &error),
    };
    let bad = match materialize_bad_bundle(root, packet, packet_rel, expected) {
        Ok(value) => value,
        Err(row) => return row,
    };
    let base_path = packet
        .get("base_fixture_path")
        .and_then(Value::as_str)
        .unwrap_or("");
    let observation = crate::red::fixture::observation::observe_materialized_with_candidate(
        root,
        store,
        validator_digests,
        packet,
        expected,
        &bad,
        base_path,
    );
    result_row(
        root,
        packet_rel,
        expected,
        &observation.error,
        if observation.ok { "pass" } else { "fail" },
        None,
        Some(&observation.check),
    )
}

fn materialize_bad_bundle(
    root: &Path,
    packet: &Value,
    packet_rel: &str,
    expected: &Value,
) -> Result<Value, Value> {
    let base_path = packet
        .get("base_fixture_path")
        .and_then(Value::as_str)
        .unwrap_or("");
    if let Some(error) = base_fixture_error(root, base_path) {
        return Err(simple_row(root, packet_rel, expected, &error));
    }
    let base = base_fixture_json_result(
        json_boundary::read_json(&root.join(base_path)),
        root,
        packet_rel,
        expected,
    )?;
    let Some(patch) = packet.get("json_patch") else {
        return Err(simple_row(
            root,
            packet_rel,
            expected,
            "red_fixture_json_patch_missing",
        ));
    };
    claim_semantics::apply_patch(&base, patch).map_err(|_| {
        simple_row(
            root,
            packet_rel,
            expected,
            "red_fixture_json_pointer_invalid",
        )
    })
}

fn simple_row(root: &Path, packet_rel: &str, expected: &Value, error: &str) -> Value {
    result_row(
        root,
        packet_rel,
        expected,
        error,
        expected_status(expected, error),
        None,
        Some(&expected_check(expected)),
    )
}

pub(crate) fn base_fixture_json_result(
    result: Result<Value, String>,
    root: &Path,
    packet_rel: &str,
    expected: &Value,
) -> Result<Value, Value> {
    result.map_err(|_| simple_row(root, packet_rel, expected, "base_fixture_malformed_json"))
}

pub(crate) struct Observation {
    pub(crate) error: String,
    pub(crate) check: String,
    pub(crate) ok: bool,
}

fn base_fixture_error(root: &Path, rel: &str) -> Option<String> {
    if !crate::red::fixture::bases::is_allowed(rel) {
        return Some("invalid_base_fixture_path".to_string());
    }
    if crate::package::inventory::package_path_error(root, rel).is_some() {
        return Some("base_fixture_path_escapes_root".to_string());
    }
    if !root.join(rel).is_file() {
        return Some("base_fixture_missing".to_string());
    }
    None
}
