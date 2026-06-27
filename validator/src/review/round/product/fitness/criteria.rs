use serde_json::Value;
use std::collections::BTreeSet;

pub(crate) fn product_impacting_claims(receipt: &Value) -> bool {
    let mut haystack = serde_json::to_string(&receipt["claim_ceiling"]).unwrap_or_default();
    haystack.push_str(&serde_json::to_string(&receipt["materiality_gate"]).unwrap_or_default());
    let haystack = haystack.to_ascii_lowercase();
    [
        "product",
        "ui",
        "install",
        "marketplace",
        "dogfood",
        "production",
        "release",
        "daily",
        "user",
        "quality",
        "fitness",
    ]
    .iter()
    .any(|term| haystack.contains(term))
}

pub(crate) fn product_claim_ids(receipt: &Value) -> BTreeSet<String> {
    receipt
        .pointer("/claim_ceiling/unsupported")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| row.get("claim_id").and_then(Value::as_str))
        .filter(|id| product_claim_id(id))
        .map(ToOwned::to_owned)
        .collect()
}

pub(crate) fn dimensions_complete(disposition: &Value) -> bool {
    let dimensions = string_set(disposition, "dimensions_checked");
    [
        "audience",
        "job",
        "context",
        "outcome",
        "accessibility",
        "cognitive_load",
        "recovery_burden",
        "continuance",
        "real_use_evidence",
        "product_success_substitution_rejection",
    ]
    .iter()
    .all(|required| dimensions.contains(*required))
}

pub(crate) fn required_substitutions() -> BTreeSet<String> {
    [
        "generic_product_simplicity_approval",
        "product::cohesion",
        "install_success",
        "package_publication",
        "first_use",
        "smoke_test",
        "fixture_pass",
        "reviewer_agreement",
        "happy_path",
        "receipt_only",
    ]
    .into_iter()
    .map(ToOwned::to_owned)
    .collect()
}

pub(crate) fn string_set(value: &Value, key: &str) -> BTreeSet<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect()
}

fn product_claim_id(id: &str) -> bool {
    [
        "product",
        "ui",
        "install",
        "marketplace",
        "dogfood",
        "production",
        "release",
        "daily",
        "user",
        "ux",
    ]
    .iter()
    .any(|term| id.contains(term))
}
