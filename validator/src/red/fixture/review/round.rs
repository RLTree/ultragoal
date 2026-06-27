use crate::red::fixture::row::{expected_check, expected_error};
use crate::red::fixtures::Observation;
use crate::schema_catalog::SchemaStore;
use serde_json::Value;
use std::path::Path;

pub(crate) fn observation(
    root: &Path,
    store: &SchemaStore,
    packet: &Value,
    expected: &Value,
    bad: &Value,
) -> Observation {
    let failures = red_errors(root, store, packet, bad);
    if !first_failure_required(packet)
        && let Some(found) = failures
            .iter()
            .find(|f| f.check == expected_check(expected) && f.error == expected_error(expected))
    {
        return Observation {
            ok: true,
            check: found.check.clone(),
            error: found.error.clone(),
        };
    }
    let first = failures.first();
    let error = first_review_error(first);
    let check = first
        .map(|failure| failure.check.clone())
        .unwrap_or_default();
    Observation {
        ok: check == expected_check(expected) && error == expected_error(expected),
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

fn red_errors(
    root: &Path,
    store: &SchemaStore,
    packet: &Value,
    bad: &Value,
) -> Vec<crate::review::round::ReviewFailure> {
    let anchors = packet
        .get("review_round_anchor_overrides")
        .and_then(Value::as_object)
        .map(|overrides| {
            let validator = overrides
                .get("validator_receipt")
                .and_then(Value::as_str)
                .unwrap_or("fixtures/review-round/anchors/validator-receipt.json");
            let review_target = overrides
                .get("review_target_receipt")
                .and_then(Value::as_str)
                .unwrap_or("fixtures/review-round/anchors/review-target-receipt.json");
            let archive = overrides
                .get("archive_receipt")
                .and_then(Value::as_str)
                .unwrap_or("fixtures/review-round/anchors/archive-receipt.json");
            crate::review::round::anchor::values::fixture_anchor_values_for(
                root,
                validator,
                review_target,
                archive,
            )
        });
    match anchors {
        Some(anchors) => crate::review::round::red_errors_with_anchors(root, store, bad, &anchors),
        None => crate::review::round::red_errors(root, store, bad),
    }
}

pub(crate) fn first_review_error(first: Option<&crate::review::round::ReviewFailure>) -> String {
    first
        .map(|failure| failure.error.clone())
        .unwrap_or_else(|| "no_failure".to_string())
}
