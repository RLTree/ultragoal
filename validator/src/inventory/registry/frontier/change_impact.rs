use super::{envelope_codec, scope_consumption};
use crate::context::query_git;
use crate::context::ReadSession;
use crate::inventory::types::InventoryError;
use serde_json::Value;

pub(super) fn validate(
    reads: &ReadSession,
    registry: &Value,
    lanes: &[Value],
    scopes: &[Value],
    record: &Value,
    lane: &Value,
    base: (&str, &str),
) -> Result<(), InventoryError> {
    let changed = required(record, "changed_set")?;
    let consumed = required(record, "consumed_set")?;
    let to = handoff_revision(record, base)?;
    envelope_codec::validate(changed, "changed", "git-diff-name-status", base, to)?;
    envelope_codec::validate(
        consumed,
        "consumed",
        "registry-scope-consumption",
        base,
        base,
    )?;
    let expected_consumed = scope_consumption::derive(registry, lanes, scopes, lane)?;
    envelope_codec::exact_categories(
        consumed,
        &expected_consumed,
        "lease consumed set differs from canonical authority",
    )?;
    scope_consumption::validate_dependencies(consumed, lane, lanes)?;
    let expected_changed = derive_changed(reads, record, base, to)?;
    envelope_codec::exact_categories(
        changed,
        &expected_changed,
        "lease changed set differs from Git",
    )?;
    envelope_codec::validate_intersection(changed, &expected_changed, &expected_consumed)?;
    envelope_codec::validate_intersection(consumed, &expected_consumed, &expected_changed)
}

fn handoff_revision<'a>(
    record: &'a Value,
    base: (&'a str, &'a str),
) -> Result<(&'a str, &'a str), InventoryError> {
    match record.pointer("/handoff/status").and_then(Value::as_str) {
        Some("unissued" | "pending") => Ok(base),
        Some("verified") => Ok((
            text(
                record.pointer("/handoff/commit"),
                "lease handoff lacks commit",
            )?,
            text(record.pointer("/handoff/tree"), "lease handoff lacks tree")?,
        )),
        _ => Err(invalid("lease handoff status is malformed")),
    }
}

fn derive_changed(
    reads: &ReadSession,
    record: &Value,
    from: (&str, &str),
    to: (&str, &str),
) -> Result<Value, InventoryError> {
    if git(reads, &["rev-parse", &format!("{}^{{tree}}", to.0)])? != to.1 {
        return Err(invalid("lease changed-set target has the wrong Git tree"));
    }
    let mut categories = envelope_codec::empty_categories();
    for path in git(
        reads,
        &["diff", "--name-only", "--no-renames", from.0, to.0],
    )?
    .lines()
    .filter(|path| !path.is_empty())
    {
        categories
            .get_mut(classify(path, record)?)
            .expect("fixed category")
            .insert(path.to_owned());
    }
    reads
        .revalidate()
        .map_err(|_| invalid("lease change derivation became stale"))?;
    Ok(envelope_codec::categories_value(categories))
}

fn classify(path: &str, record: &Value) -> Result<&'static str, InventoryError> {
    for (field, category) in [
        ("generated_outputs", "generated_outputs"),
        ("fixtures", "fixtures"),
        ("owned_files", "files"),
    ] {
        if values(record.get(field))?
            .iter()
            .any(|root| overlaps(path, root))
        {
            return Ok(category);
        }
    }
    Err(invalid("lease changed path has unknown classification"))
}

fn required<'a>(record: &'a Value, field: &str) -> Result<&'a Value, InventoryError> {
    record
        .get(field)
        .ok_or_else(|| invalid(&format!("lease {field} is missing")))
}
fn values(value: Option<&Value>) -> Result<Vec<String>, InventoryError> {
    value
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("lease category is missing"))?
        .iter()
        .map(|value| text(Some(value), "lease category is malformed").map(ToOwned::to_owned))
        .collect()
}
fn text<'a>(value: Option<&'a Value>, message: &str) -> Result<&'a str, InventoryError> {
    value
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(message))
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
fn git(reads: &ReadSession, args: &[&str]) -> Result<String, InventoryError> {
    String::from_utf8(
        query_git(reads, args).map_err(|_| invalid("cannot derive lease changes from Git"))?,
    )
    .map(|text| text.trim().to_owned())
    .map_err(|_| invalid("lease change derivation is not UTF-8"))
}
fn invalid(message: &str) -> InventoryError {
    InventoryError::InvalidRegistry(message.to_owned())
}
