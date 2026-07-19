use crate::context::ReadSession;
use crate::digest;
use crate::inventory::types::InventoryError;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

const NAMES: [&str; 5] = [
    "files",
    "symbols",
    "generated_outputs",
    "fixtures",
    "effects",
];

pub(super) fn validate(
    reads: &ReadSession,
    root: &Path,
    registry: &Value,
    lanes: &[Value],
    scopes: &[Value],
    record: &Value,
    lane: &Value,
    base: (&str, &str),
) -> Result<(), InventoryError> {
    let changed = record
        .get("changed_set")
        .ok_or_else(|| invalid("lease changed set is missing"))?;
    let consumed = record
        .get("consumed_set")
        .ok_or_else(|| invalid("lease consumed set is missing"))?;
    let to = handoff_revision(record, base)?;
    validate_envelope(changed, "changed", "git-diff-name-status", base, to)?;
    validate_envelope(
        consumed,
        "consumed",
        "registry-scope-consumption",
        base,
        base,
    )?;
    let expected_consumed = consumption(registry, lanes, scopes, lane)?;
    exact_categories(
        consumed,
        &expected_consumed,
        "lease consumed set differs from canonical authority",
    )?;
    exact_dependencies(consumed, lane, lanes)?;
    let expected_changed = changed_paths(reads, root, record, base, to)?;
    exact_categories(
        changed,
        &expected_changed,
        "lease changed set differs from Git",
    )?;
    validate_intersection(changed, &expected_changed, &expected_consumed)?;
    validate_intersection(consumed, &expected_consumed, &expected_changed)?;
    Ok(())
}

fn handoff_revision<'a>(
    record: &'a Value,
    base: (&'a str, &'a str),
) -> Result<(&'a str, &'a str), InventoryError> {
    match record.pointer("/handoff/status").and_then(Value::as_str) {
        Some("unissued" | "pending") => Ok(base),
        Some("verified") => Ok((
            text(
                record.pointer("/handoff/commit").unwrap_or(&Value::Null),
                "lease handoff lacks commit",
            )?,
            text(
                record.pointer("/handoff/tree").unwrap_or(&Value::Null),
                "lease handoff lacks tree",
            )?,
        )),
        _ => Err(invalid("lease handoff status is malformed")),
    }
}

fn validate_envelope(
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
    let categories = categories(envelope)?;
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
    let aggregate = digest::canonical_json(&json!({
        "kind": kind, "algorithm": algorithm, "version": "v1",
        "from": {"commit": from.0, "tree": from.1},
        "to": {"commit": to.0, "tree": to.1}, "categories": categories,
        "category_digests": envelope.get("category_digests"),
    }));
    if envelope.get("aggregate_digest").and_then(Value::as_str) != Some(&aggregate) {
        return Err(invalid("lease envelope aggregate digest is stale"));
    }
    Ok(())
}

fn consumption(
    registry: &Value,
    lanes: &[Value],
    scopes: &[Value],
    lane: &Value,
) -> Result<Value, InventoryError> {
    let mut values = empty_categories();
    add(
        &mut values,
        "files",
        strings(registry.pointer("/root_freeze/permitted_root_paths"))?,
    )?;
    for dependency in strings(lane.get("dependencies"))? {
        let dependency = find(lanes, "id", &dependency, "lease dependency is missing")?;
        for scope_id in strings(dependency.get("scope_ids"))? {
            let scope = find(scopes, "scope_id", &scope_id, "dependency scope is missing")?;
            add(&mut values, "files", strings(scope.get("contract_roots"))?)?;
            add(&mut values, "files", strings(scope.get("owned_roots"))?)?;
            add(
                &mut values,
                "generated_outputs",
                strings(scope.get("generated_roots"))?,
            )?;
            add(
                &mut values,
                "fixtures",
                strings(scope.get("fixture_roots"))?,
            )?;
            add(&mut values, "symbols", strings(scope.get("owned_symbols"))?)?;
            add(&mut values, "effects", strings(scope.get("effects"))?)?;
        }
    }
    Ok(Value::Object(
        values
            .into_iter()
            .map(|(name, rows)| {
                (
                    name.to_owned(),
                    Value::Array(rows.into_iter().map(Value::String).collect()),
                )
            })
            .collect(),
    ))
}

fn exact_dependencies(
    consumed: &Value,
    lane: &Value,
    lanes: &[Value],
) -> Result<(), InventoryError> {
    let expected = strings(lane.get("dependencies"))?
        .into_iter()
        .map(|id| {
            find(lanes, "id", &id, "lease dependency is missing")?
                .get("current_identity")
                .cloned()
                .filter(|value| !value.is_null())
                .ok_or_else(|| invalid("lane dependency identity is missing"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if consumed
        .get("dependency_identities")
        .and_then(Value::as_array)
        != Some(&expected)
    {
        return Err(invalid("lease dependency identities are stale"));
    }
    Ok(())
}

fn changed_paths(
    reads: &ReadSession,
    root: &Path,
    record: &Value,
    from: (&str, &str),
    to: (&str, &str),
) -> Result<Value, InventoryError> {
    let tree = git(root, &["rev-parse", &format!("{}^{{tree}}", to.0)])?;
    if tree != to.1 {
        return Err(invalid("lease changed-set target has the wrong Git tree"));
    }
    let output = git(root, &["diff", "--name-only", "--no-renames", from.0, to.0])?;
    let mut values = empty_categories();
    for path in output.lines().filter(|path| !path.is_empty()) {
        let category = classify(path, record)?;
        values
            .get_mut(category)
            .expect("fixed category")
            .insert(path.to_owned());
    }
    reads
        .revalidate()
        .map_err(|_| invalid("lease change derivation became stale"))?;
    Ok(Value::Object(
        values
            .into_iter()
            .map(|(name, rows)| {
                (
                    name.to_owned(),
                    Value::Array(rows.into_iter().map(Value::String).collect()),
                )
            })
            .collect(),
    ))
}

fn classify<'a>(path: &str, record: &'a Value) -> Result<&'static str, InventoryError> {
    for (field, category) in [
        ("generated_outputs", "generated_outputs"),
        ("fixtures", "fixtures"),
        ("owned_files", "files"),
    ] {
        if strings(record.get(field))?
            .iter()
            .any(|root| overlaps(path, root))
        {
            return Ok(category);
        }
    }
    Err(invalid("lease changed path has unknown classification"))
}

fn validate_intersection(
    changed: &Value,
    changed_values: &Value,
    consumed_values: &Value,
) -> Result<(), InventoryError> {
    let actual = changed
        .get("intersection")
        .ok_or_else(|| invalid("lease intersection disposition is missing"))?;
    if actual.get("status").and_then(Value::as_str) == Some("unknown") {
        return Err(invalid("lease intersection disposition is unknown"));
    }
    let expected = intersection(changed_values, consumed_values)?;
    if actual != &expected {
        return Err(invalid("lease intersection disposition is stale"));
    }
    Ok(())
}

fn intersection(changed: &Value, consumed: &Value) -> Result<Value, InventoryError> {
    let mut matches = Vec::new();
    for name in NAMES {
        let left = strings(changed.get(name))?;
        let right = if matches!(name, "files" | "generated_outputs" | "fixtures") {
            ["files", "generated_outputs", "fixtures"]
                .into_iter()
                .flat_map(|key| strings(consumed.get(key)).unwrap_or_default())
                .collect()
        } else {
            strings(consumed.get(name))?
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
    Ok(
        json!({"status": if matches.is_empty() { "none" } else { "intersects" }, "categories": matches}),
    )
}

fn exact_categories(
    envelope: &Value,
    expected: &Value,
    message: &str,
) -> Result<(), InventoryError> {
    if envelope.get("categories") != Some(expected) {
        return Err(invalid(message));
    }
    Ok(())
}

fn categories(envelope: &Value) -> Result<&serde_json::Map<String, Value>, InventoryError> {
    envelope
        .get("categories")
        .and_then(Value::as_object)
        .ok_or_else(|| invalid("lease envelope categories are missing"))
}
fn empty_categories() -> std::collections::BTreeMap<&'static str, BTreeSet<String>> {
    NAMES
        .into_iter()
        .map(|name| (name, BTreeSet::new()))
        .collect()
}
fn add(
    values: &mut std::collections::BTreeMap<&'static str, BTreeSet<String>>,
    name: &'static str,
    rows: Vec<String>,
) -> Result<(), InventoryError> {
    values
        .get_mut(name)
        .ok_or_else(|| invalid("lease category is unknown"))?
        .extend(rows);
    Ok(())
}
fn strings(value: Option<&Value>) -> Result<Vec<String>, InventoryError> {
    value
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("lease category is missing"))?
        .iter()
        .map(|value| text(value, "lease category is malformed").map(ToOwned::to_owned))
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
        text(
            value.get("commit").unwrap_or(&Value::Null),
            "lease envelope commit is missing",
        )?,
        text(
            value.get("tree").unwrap_or(&Value::Null),
            "lease envelope tree is missing",
        )?,
    ))
}
fn find<'a>(
    rows: &'a [Value],
    field: &str,
    value: &str,
    message: &str,
) -> Result<&'a Value, InventoryError> {
    rows.iter()
        .find(|row| row.get(field).and_then(Value::as_str) == Some(value))
        .ok_or_else(|| invalid(message))
}
fn text<'a>(value: &'a Value, message: &str) -> Result<&'a str, InventoryError> {
    value.as_str().ok_or_else(|| invalid(message))
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
fn git(root: &Path, args: &[&str]) -> Result<String, InventoryError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|_| invalid("cannot derive lease changes from Git"))?;
    if !output.status.success() {
        return Err(invalid("cannot derive lease changes from Git"));
    }
    String::from_utf8(output.stdout)
        .map(|text| text.trim().to_owned())
        .map_err(|_| invalid("lease change derivation is not UTF-8"))
}
fn invalid(message: &str) -> InventoryError {
    InventoryError::InvalidRegistry(message.to_owned())
}
