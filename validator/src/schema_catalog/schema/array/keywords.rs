use crate::schema_catalog::SchemaStore;
use serde_json::Value;

pub(crate) fn check(
    store: &SchemaStore,
    root: &Value,
    schema: &Value,
    instance: &Value,
    path: &str,
    errors: &mut Vec<String>,
    depth: usize,
) {
    let Some(items) = instance.as_array() else {
        return;
    };
    item_count_checks(schema, items, path, errors);
    if schema.get("uniqueItems").and_then(Value::as_bool) == Some(true) {
        unique_items_check(items, path, errors);
    }
    item_schema_checks(store, root, schema, items, path, errors, depth);
    contains_check(store, root, schema, items, path, errors, depth);
}

fn item_count_checks(schema: &Value, items: &[Value], path: &str, errors: &mut Vec<String>) {
    if let Some(min) = schema.get("minItems").and_then(Value::as_u64)
        && items.len() < min as usize
    {
        errors.push(format!("{path}: minItems"));
    }
    if let Some(max) = schema.get("maxItems").and_then(Value::as_u64)
        && items.len() > max as usize
    {
        errors.push(format!("{path}: maxItems"));
    }
}

fn item_schema_checks(
    store: &SchemaStore,
    root: &Value,
    schema: &Value,
    items: &[Value],
    path: &str,
    errors: &mut Vec<String>,
    depth: usize,
) {
    if let Some(item_schema) = schema.get("items") {
        for (index, value) in items.iter().enumerate() {
            crate::schema_catalog::schema::keywords::validate_at(
                store,
                root,
                item_schema,
                value,
                &format!("{path}[{index}]"),
                errors,
                depth + 1,
            );
        }
    }
}

fn contains_check(
    store: &SchemaStore,
    root: &Value,
    schema: &Value,
    items: &[Value],
    path: &str,
    errors: &mut Vec<String>,
    depth: usize,
) {
    if let Some(contains) = schema.get("contains") {
        let min = schema
            .get("minContains")
            .and_then(Value::as_u64)
            .unwrap_or(1) as usize;
        let max = schema
            .get("maxContains")
            .and_then(Value::as_u64)
            .map(|value| value as usize);
        let stop_after = max.map(|limit| limit.saturating_add(1)).unwrap_or(min);
        let matches = fast_const_contains_count(contains, items, stop_after).unwrap_or_else(|| {
            slow_contains_count(store, root, contains, items, path, depth, stop_after)
        });
        if matches < min {
            errors.push(format!("{path}: contains mismatch"));
        }
        if max.is_some_and(|limit| matches > limit) {
            errors.push(format!("{path}: maxContains"));
        }
    }
}

fn fast_const_contains_count(schema: &Value, items: &[Value], stop_after: usize) -> Option<usize> {
    let object = schema.as_object()?;
    if object
        .get("type")
        .and_then(Value::as_str)
        .is_some_and(|t| t != "object")
    {
        return None;
    }
    let required = object.get("required").and_then(Value::as_array)?;
    let properties = object.get("properties").and_then(Value::as_object)?;
    let required_consts = properties
        .iter()
        .filter_map(|(key, subschema)| {
            let has_required = required.iter().any(|value| value.as_str() == Some(key));
            subschema
                .get("const")
                .filter(|_| has_required)
                .map(|v| (key, v))
        })
        .collect::<Vec<_>>();
    if required_consts.is_empty() || required_consts.len() != required.len() {
        return None;
    }
    let mut matches = 0;
    for item in items {
        let Some(obj) = item.as_object() else {
            continue;
        };
        if required_consts
            .iter()
            .all(|(key, expected)| obj.get(*key) == Some(*expected))
        {
            matches += 1;
            if stop_after > 0 && matches >= stop_after {
                break;
            }
        }
    }
    Some(matches)
}

fn slow_contains_count(
    store: &SchemaStore,
    root: &Value,
    contains: &Value,
    items: &[Value],
    path: &str,
    depth: usize,
    stop_after: usize,
) -> usize {
    let mut matches = 0;
    for value in items {
        let mut nested = Vec::new();
        crate::schema_catalog::schema::keywords::validate_at(
            store,
            root,
            contains,
            value,
            path,
            &mut nested,
            depth + 1,
        );
        if nested.is_empty() {
            matches += 1;
            if stop_after > 0 && matches >= stop_after {
                break;
            }
        }
    }
    matches
}

fn unique_items_check(items: &[Value], path: &str, errors: &mut Vec<String>) {
    let mut seen = std::collections::BTreeSet::new();
    if items.iter().any(|item| !seen.insert(item.to_string())) {
        errors.push(format!("{path}: uniqueItems"));
    }
}
