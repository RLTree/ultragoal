use crate::inventory::types::InventoryError;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const ROOT_ONLY: &[&str] = &[
    ".agents/",
    ".codex/",
    ".git/",
    "Cargo.lock",
    "Cargo.toml",
    "COMPLETION_MANIFEST.json",
    "LANE_REGISTRY.json",
    "VERIFICATION_BACKLOG.json",
    "docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/",
    "generated/",
    "migration/",
    "schemas/",
    "validator/src/cli/successor_public/",
];

pub(super) fn validate(registry: &Value) -> Result<(), InventoryError> {
    let mappings = registry
        .get("scope_mappings")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("missing scope mappings"))?;
    let mut paths = Vec::new();
    let mut symbols = BTreeMap::new();
    let mut effects = BTreeMap::new();
    for mapping in mappings {
        let scope = text(mapping, "scope_id")?;
        let forbidden = strings(mapping, "forbidden_roots", true)?;
        if !ROOT_ONLY.iter().all(|root| forbidden.contains(*root)) {
            return Err(invalid("scope omits a root-only surface"));
        }
        for field in ["owned_roots", "generated_roots", "fixture_roots"] {
            for path in strings(mapping, field, false)? {
                validate_path(path)?;
                if forbidden.iter().any(|root| overlaps(path, root)) {
                    return Err(invalid("scope owns a root-only surface"));
                }
                paths.push((scope, path));
            }
        }
        register_unique(
            &mut symbols,
            scope,
            strings(mapping, "owned_symbols", true)?,
            "symbol",
        )?;
        register_unique(
            &mut effects,
            scope,
            strings(mapping, "effects", true)?,
            "effect",
        )?;
    }
    for (index, (left_scope, left)) in paths.iter().enumerate() {
        for (right_scope, right) in paths.iter().skip(index + 1) {
            if left_scope != right_scope && overlaps(left, right) {
                return Err(invalid("scope path authority overlaps"));
            }
        }
    }
    Ok(())
}

fn register_unique<'a>(
    seen: &mut BTreeMap<&'a str, &'a str>,
    scope: &'a str,
    values: BTreeSet<&'a str>,
    kind: &str,
) -> Result<(), InventoryError> {
    for value in values {
        if value.trim() != value || value.is_empty() {
            return Err(invalid(&format!("scope {kind} is not normalized")));
        }
        if seen
            .insert(value, scope)
            .is_some_and(|owner| owner != scope)
        {
            return Err(invalid(&format!("scope {kind} authority overlaps")));
        }
    }
    Ok(())
}

fn strings<'a>(
    value: &'a Value,
    key: &str,
    require_nonempty: bool,
) -> Result<BTreeSet<&'a str>, InventoryError> {
    let values = value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| invalid(&format!("scope lacks {key}")))?;
    if require_nonempty && values.is_empty() {
        return Err(invalid(&format!("scope has empty {key}")));
    }
    let parsed = values
        .iter()
        .map(|item| {
            item.as_str()
                .ok_or_else(|| invalid(&format!("scope has invalid {key}")))
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    if parsed.len() != values.len() {
        return Err(invalid(&format!("scope has duplicate {key}")));
    }
    Ok(parsed)
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, InventoryError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(&format!("scope lacks {key}")))
}

fn validate_path(path: &str) -> Result<(), InventoryError> {
    let normalized = path.trim_end_matches('/');
    if normalized.is_empty()
        || path.starts_with('/')
        || normalized
            .split('/')
            .any(|part| matches!(part, "" | "." | ".."))
    {
        return Err(invalid("scope path is not normalized"));
    }
    Ok(())
}

fn overlaps(left: &str, right: &str) -> bool {
    let left = left.trim_end_matches('/');
    let right = right.trim_end_matches('/');
    left == right
        || left
            .strip_prefix(right)
            .is_some_and(|suffix| suffix.starts_with('/'))
        || right
            .strip_prefix(left)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn invalid(message: &str) -> InventoryError {
    InventoryError::InvalidRegistry(message.to_owned())
}
