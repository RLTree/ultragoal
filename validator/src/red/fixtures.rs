use crate::claim_semantics;
use crate::json_boundary;
use crate::red::fixture::row::{invalid_row, result_row};
use crate::schema_catalog::SchemaStore;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

pub(crate) use crate::red::fixture::materialization::{Observation, base_fixture_json_result};
use crate::red::fixture::materialization::{base_fixture_error, simple_row};

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
    let target_digest = crate::package::inventory::package_digest(root).unwrap_or_default();
    let mut runtime_digest_cache = BTreeMap::new();
    let runtime_red_fixture_ids = crate::audit::artifacts::safe_red_ids(root);
    let runtime_red_fixtures = crate::red::fixture::runtime::receipt::red_fixtures_with_ids(
        root,
        &runtime_red_fixture_ids,
        &mut runtime_digest_cache,
    );
    let runtime_input_digests = crate::red::fixture::runtime::receipt::input_digests_with_cache(
        root,
        &mut runtime_digest_cache,
    );
    let mut semantic_cache = crate::claim_semantics::SemanticCache::default();
    let mut results = BTreeMap::new();
    for row in items {
        let row_id = row
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("invalid-row");
        let result = result_for_row(
            root,
            store,
            validator_digests,
            row,
            &target_digest,
            &mut runtime_digest_cache,
            &runtime_red_fixture_ids,
            &runtime_red_fixtures,
            &runtime_input_digests,
            &mut semantic_cache,
        );
        results.insert(row_id.to_string(), result);
    }
    results
}

fn result_for_row(
    root: &Path,
    store: &SchemaStore,
    validator_digests: &BTreeMap<String, String>,
    row: &Value,
    target_digest: &str,
    runtime_digest_cache: &mut BTreeMap<String, String>,
    runtime_red_fixture_ids: &[String],
    runtime_red_fixtures: &Value,
    runtime_input_digests: &[Value],
    semantic_cache: &mut crate::claim_semantics::SemanticCache,
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
    materialized_result(
        root,
        store,
        validator_digests,
        &packet,
        packet_rel,
        target_digest,
        runtime_digest_cache,
        runtime_red_fixture_ids,
        runtime_red_fixtures,
        runtime_input_digests,
        semantic_cache,
    )
}

fn materialized_result(
    root: &Path,
    store: &SchemaStore,
    validator_digests: &BTreeMap<String, String>,
    packet: &Value,
    packet_rel: &str,
    target_digest: &str,
    runtime_digest_cache: &mut BTreeMap<String, String>,
    runtime_red_fixture_ids: &[String],
    runtime_red_fixtures: &Value,
    runtime_input_digests: &[Value],
    semantic_cache: &mut crate::claim_semantics::SemanticCache,
) -> Value {
    let expected = &packet["expected_failure"];
    let _filesystem_guard = match crate::red::filesystem::fixtures::materialize(root, packet) {
        Ok(guard) => guard,
        Err(error) => return simple_row(root, packet_rel, expected, &error),
    };
    let bad = match materialize_bad_bundle(
        root,
        packet,
        packet_rel,
        expected,
        validator_digests,
        target_digest,
        runtime_digest_cache,
        runtime_red_fixture_ids,
        runtime_red_fixtures,
        runtime_input_digests,
    ) {
        Ok(value) => value,
        Err(row) => return row,
    };
    let base_path = packet
        .get("base_fixture_path")
        .and_then(Value::as_str)
        .unwrap_or("");
    let observation = crate::red::fixture::observation::observe_materialized_with_candidate_cached(
        root,
        store,
        validator_digests,
        packet,
        expected,
        &bad,
        base_path,
        target_digest,
        semantic_cache,
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
    validator_digests: &BTreeMap<String, String>,
    target_digest: &str,
    runtime_digest_cache: &mut BTreeMap<String, String>,
    runtime_red_fixture_ids: &[String],
    runtime_red_fixtures: &Value,
    runtime_input_digests: &[Value],
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
    let base = crate::red::fixture::runtime::receipt::bind_with_candidate_and_cache(
        root,
        &base,
        validator_digests,
        target_digest,
        runtime_digest_cache,
        runtime_red_fixture_ids,
        runtime_red_fixtures,
        runtime_input_digests,
    );
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

#[cfg(test)]
pub(crate) fn runtime_bound_bundle(
    root: &Path,
    value: &Value,
    validator_digests: &BTreeMap<String, String>,
) -> Value {
    crate::red::fixture::runtime::receipt::bind(root, value, validator_digests)
}
