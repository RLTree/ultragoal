use crate::digest;
use crate::inventory::types::InventoryError;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub(super) const NAMES: [&str; 5] = [
    "files",
    "symbols",
    "generated_outputs",
    "fixtures",
    "effects",
];
pub(super) type Categories = BTreeMap<&'static str, BTreeSet<String>>;

pub(super) fn validate(
    envelope: &Value,
    kind: &str,
    algorithm: &str,
    from: (&str, &str),
    to: (&str, &str),
) -> Result<(), InventoryError> {
    if envelope.get("kind").and_then(Value::as_str) != Some(kind)
        || envelope.get("algorithm").and_then(Value::as_str) != Some(algorithm)
        || envelope.get("version").and_then(Value::as_str) != Some("v1")
        || revision(envelope.get("from"))? != from
        || revision(envelope.get("to"))? != to
    {
        return Err(invalid("lease envelope identity is not current"));
    }
    if envelope.get("classification").and_then(Value::as_str) != Some("classified") {
        return Err(invalid("lease envelope classification is unknown"));
    }
    let categories = object(envelope)?;
    for name in NAMES {
        let values = categories
            .get(name)
            .and_then(Value::as_array)
            .ok_or_else(|| invalid("lease envelope category is malformed"))?;
        if !normalized(values) {
            return Err(invalid("lease envelope category is not normalized"));
        }
        if envelope
            .pointer(&format!("/category_digests/{name}"))
            .and_then(Value::as_str)
            != Some(&digest::canonical_json(&Value::Array(values.clone())))
        {
            return Err(invalid("lease envelope category digest is stale"));
        }
    }
    let aggregate = digest::canonical_json(
        &json!({"kind":kind,"algorithm":algorithm,"version":"v1","from":{"commit":from.0,"tree":from.1},"to":{"commit":to.0,"tree":to.1},"categories":categories,"category_digests":envelope.get("category_digests")}),
    );
    if envelope.get("aggregate_digest").and_then(Value::as_str) != Some(&aggregate) {
        return Err(invalid("lease envelope aggregate digest is stale"));
    }
    Ok(())
}

pub(super) fn exact_categories(
    envelope: &Value,
    expected: &Value,
    message: &str,
) -> Result<(), InventoryError> {
    if envelope.get("categories") != Some(expected) {
        return Err(invalid(message));
    }
    Ok(())
}
pub(super) fn validate_intersection(
    envelope: &Value,
    left: &Value,
    right: &Value,
) -> Result<(), InventoryError> {
    let actual = envelope
        .get("intersection")
        .ok_or_else(|| invalid("lease intersection disposition is missing"))?;
    if actual.get("status").and_then(Value::as_str) == Some("unknown") {
        return Err(invalid("lease intersection disposition is unknown"));
    }
    if actual != &intersection(left, right)? {
        return Err(invalid("lease intersection disposition is stale"));
    }
    Ok(())
}
pub(super) fn empty_categories() -> Categories {
    NAMES
        .into_iter()
        .map(|name| (name, BTreeSet::new()))
        .collect()
}
pub(super) fn categories_value(categories: Categories) -> Value {
    Value::Object(
        categories
            .into_iter()
            .map(|(name, rows)| {
                (
                    name.to_owned(),
                    Value::Array(rows.into_iter().map(Value::String).collect()),
                )
            })
            .collect(),
    )
}

fn intersection(left: &Value, right: &Value) -> Result<Value, InventoryError> {
    let mut matches = Vec::new();
    for name in NAMES {
        let left = values(left.get(name))?;
        let right = if matches!(name, "files" | "generated_outputs" | "fixtures") {
            ["files", "generated_outputs", "fixtures"]
                .into_iter()
                .flat_map(|key| values(right.get(key)).unwrap_or_default())
                .collect()
        } else {
            values(right.get(name))?
        };
        if left.iter().any(|value| {
            right.iter().any(|other| {
                if name == "symbols" {
                    namespace_overlaps(value, other)
                } else if name == "effects" {
                    value == other
                } else {
                    overlaps(value, other)
                }
            })
        }) {
            matches.push(name);
        }
    }
    Ok(json!({"status":if matches.is_empty(){"none"}else{"intersects"},"categories":matches}))
}
fn object(envelope: &Value) -> Result<&serde_json::Map<String, Value>, InventoryError> {
    envelope
        .get("categories")
        .and_then(Value::as_object)
        .ok_or_else(|| invalid("lease envelope categories are missing"))
}
fn values(value: Option<&Value>) -> Result<Vec<String>, InventoryError> {
    value
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("lease category is missing"))?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(ToOwned::to_owned)
                .ok_or_else(|| invalid("lease category is malformed"))
        })
        .collect()
}
fn normalized(values: &[Value]) -> bool {
    let rows = values.iter().filter_map(Value::as_str).collect::<Vec<_>>();
    rows.len() == values.len()
        && rows.windows(2).all(|pair| pair[0] < pair[1])
        && rows.iter().all(|row| !row.is_empty() && row.trim() == *row)
}
fn revision(value: Option<&Value>) -> Result<(&str, &str), InventoryError> {
    let value = value.ok_or_else(|| invalid("lease envelope revision is missing"))?;
    Ok((
        value
            .get("commit")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid("lease envelope commit is missing"))?,
        value
            .get("tree")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid("lease envelope tree is missing"))?,
    ))
}
fn overlaps(left: &str, right: &str) -> bool {
    let left = left.trim_end_matches('/');
    let right = right.trim_end_matches('/');
    left == right
        || left
            .strip_prefix(right)
            .is_some_and(|tail| tail.starts_with('/'))
        || right
            .strip_prefix(left)
            .is_some_and(|tail| tail.starts_with('/'))
}
fn namespace_overlaps(left: &str, right: &str) -> bool {
    left == right
        || left
            .strip_prefix(right)
            .is_some_and(|tail| tail.starts_with("::"))
        || right
            .strip_prefix(left)
            .is_some_and(|tail| tail.starts_with("::"))
}
fn invalid(message: &str) -> InventoryError {
    InventoryError::InvalidRegistry(message.to_owned())
}
