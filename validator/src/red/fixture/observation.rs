use crate::claim_semantics;
use crate::red::fixture::row::{expected_check, expected_error};
use crate::red::fixtures::Observation;
use crate::schema_catalog::{self, SchemaStore};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

pub(crate) fn observe_materialized(
    root: &Path,
    store: &SchemaStore,
    validator_digests: &BTreeMap<String, String>,
    packet: &Value,
    expected: &Value,
    bad: &Value,
    base_path: &str,
) -> Observation {
    if crate::review::round::is_review_round_fixture(base_path) {
        return crate::red::fixture::review::round::observation(root, store, packet, expected, bad);
    }
    if let Some(observation) =
        crate::red::fixture::package::observation(root, expected, bad, base_path)
    {
        return observation;
    }
    let schema_errors = crate::red::fixture::schema::errors(store, packet, bad);
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
    )
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
) -> Observation {
    let failures = claim_semantics::semantic_failures(bad, root, validator_digests);
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
