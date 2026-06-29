use crate::claim_semantics;
use crate::red::fixture::row::{expected_check, expected_error};
use crate::red::fixtures::Observation;
use crate::schema_catalog::{self, SchemaStore};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

#[cfg(test)]
pub(crate) fn observe_materialized(
    root: &Path,
    store: &SchemaStore,
    validator_digests: &BTreeMap<String, String>,
    packet: &Value,
    expected: &Value,
    bad: &Value,
    base_path: &str,
) -> Observation {
    let target_digest = crate::package::inventory::package_digest(root).unwrap_or_default();
    observe_materialized_with_candidate(
        root,
        store,
        validator_digests,
        packet,
        expected,
        bad,
        base_path,
        &target_digest,
    )
}

#[cfg(test)]
pub(crate) fn observe_materialized_with_candidate(
    root: &Path,
    store: &SchemaStore,
    validator_digests: &BTreeMap<String, String>,
    packet: &Value,
    expected: &Value,
    bad: &Value,
    base_path: &str,
    target_digest: &str,
) -> Observation {
    let mut semantic_cache = claim_semantics::SemanticCache::default();
    observe_materialized_with_candidate_cached(
        root,
        store,
        validator_digests,
        packet,
        expected,
        bad,
        base_path,
        target_digest,
        &mut semantic_cache,
    )
}

pub(crate) fn observe_materialized_with_candidate_cached(
    root: &Path,
    store: &SchemaStore,
    validator_digests: &BTreeMap<String, String>,
    packet: &Value,
    expected: &Value,
    bad: &Value,
    base_path: &str,
    target_digest: &str,
    semantic_cache: &mut claim_semantics::SemanticCache,
) -> Observation {
    if crate::review::round::is_review_round_fixture(base_path) {
        return crate::red::fixture::review::round::observation(root, store, packet, expected, bad);
    }
    let package_observation = if has_filesystem_fixtures(packet) {
        let mut cache = crate::red::fixture::package::ObservationCache::default();
        crate::red::fixture::package::observation_with_candidate_cached(
            root,
            expected,
            bad,
            base_path,
            target_digest,
            &mut cache,
        )
    } else {
        crate::red::fixture::package::observation_with_candidate_cached(
            root,
            expected,
            bad,
            base_path,
            target_digest,
            &mut semantic_cache.package_observations,
        )
    };
    if let Some(observation) = package_observation {
        return observation;
    }
    let schema_errors = if schema_validation_required(packet, expected) {
        crate::red::fixture::schema::errors(store, packet, bad)
    } else {
        Vec::new()
    };
    if packet
        .pointer("/materialization/expected_validation_layer")
        .and_then(Value::as_str)
        == Some("schema")
    {
        return schema_observation(packet, expected, &schema_errors);
    }
    semantic_observation(
        root,
        validator_digests,
        packet,
        expected,
        bad,
        &schema_errors,
        semantic_cache,
    )
}

fn schema_validation_required(packet: &Value, expected: &Value) -> bool {
    let expected_layer_is_schema = packet
        .pointer("/materialization/expected_validation_layer")
        .and_then(Value::as_str)
        == Some("schema");
    expected_layer_is_schema
        || expected_check(expected) == "schema-valid"
        || packet
            .pointer("/materialization/post_patch_schema_valid")
            .and_then(Value::as_bool)
            == Some(false)
}

fn has_filesystem_fixtures(packet: &Value) -> bool {
    packet
        .get("filesystem_fixtures")
        .and_then(Value::as_array)
        .is_some_and(|fixtures| !fixtures.is_empty())
}

#[cfg(test)]
pub(crate) fn schema_validation_required_for_test(packet: &Value, expected: &Value) -> bool {
    schema_validation_required(packet, expected)
}

fn schema_observation(packet: &Value, expected: &Value, schema_errors: &[String]) -> Observation {
    let error = schema_catalog::schema_error_code(schema_errors);
    let post_valid = packet
        .pointer("/materialization/post_patch_schema_valid")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    Observation {
        check: expected_check(expected),
        ok: !schema_errors.is_empty() && error == expected_error(expected) && !post_valid,
        error,
    }
}

fn semantic_observation(
    root: &Path,
    validator_digests: &BTreeMap<String, String>,
    packet: &Value,
    expected: &Value,
    bad: &Value,
    schema_errors: &[String],
    semantic_cache: &mut claim_semantics::SemanticCache,
) -> Observation {
    let failures =
        claim_semantics::semantic_failures_with_cache(bad, root, validator_digests, semantic_cache);
    if !first_failure_required(packet)
        && let Some(found) = failures
            .iter()
            .find(|f| f.check_id == expected_check(expected) && f.error == expected_error(expected))
    {
        return Observation {
            ok: schema_errors.is_empty(),
            check: found.check_id.clone(),
            error: found.error.clone(),
        };
    }
    let first = failures.first();
    let error = first_semantic_error(first);
    let check = first.map(|f| f.check_id.clone()).unwrap_or_default();
    Observation {
        ok: check == expected_check(expected)
            && error == expected_error(expected)
            && schema_errors.is_empty(),
        check,
        error,
    }
}

fn first_failure_required(packet: &Value) -> bool {
    packet
        .pointer("/materialization/first_failure_must_match_expected")
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

pub(crate) fn first_semantic_error(first: Option<&crate::audit::contract::Failure>) -> String {
    first
        .map(|failure| failure.error.clone())
        .unwrap_or_else(|| "no_failure".to_string())
}
