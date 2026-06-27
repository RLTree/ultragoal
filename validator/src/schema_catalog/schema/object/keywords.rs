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
    let Some(object) = instance.as_object() else {
        return;
    };
    property_count_checks(schema, object, path, errors);
    property_names_checks(store, root, schema, object, path, errors, depth);
    required_checks(schema, object, path, errors);
    if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
        for (key, subschema) in properties {
            if let Some(value) = object.get(key) {
                crate::schema_catalog::schema::keywords::validate_at(
                    store,
                    root,
                    subschema,
                    value,
                    &join(path, key),
                    errors,
                    depth + 1,
                );
            }
        }
    }
    additional_property_checks(store, root, schema, object, path, errors, depth);
}

fn property_count_checks(
    schema: &Value,
    object: &serde_json::Map<String, Value>,
    path: &str,
    errors: &mut Vec<String>,
) {
    if let Some(min) = schema.get("minProperties").and_then(Value::as_u64)
        && object.len() < min as usize
    {
        errors.push(format!("{path}: minProperties"));
    }
    if let Some(max) = schema.get("maxProperties").and_then(Value::as_u64)
        && object.len() > max as usize
    {
        errors.push(format!("{path}: maxProperties"));
    }
}

fn property_names_checks(
    store: &SchemaStore,
    root: &Value,
    schema: &Value,
    object: &serde_json::Map<String, Value>,
    path: &str,
    errors: &mut Vec<String>,
    depth: usize,
) {
    let Some(name_schema) = schema.get("propertyNames") else {
        return;
    };
    for key in object.keys() {
        crate::schema_catalog::schema::keywords::validate_at(
            store,
            root,
            name_schema,
            &Value::String(key.clone()),
            &format!("{path}.{key}"),
            errors,
            depth + 1,
        );
    }
}

fn required_checks(
    schema: &Value,
    object: &serde_json::Map<String, Value>,
    path: &str,
    errors: &mut Vec<String>,
) {
    for key in schema
        .get("required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(name) = key.as_str()
            && !object.contains_key(name)
        {
            errors.push(format!("{path}.{name}: required"));
        }
    }
}

fn additional_property_checks(
    store: &SchemaStore,
    root: &Value,
    schema: &Value,
    object: &serde_json::Map<String, Value>,
    path: &str,
    errors: &mut Vec<String>,
    depth: usize,
) {
    let known = schema.get("properties").and_then(Value::as_object);
    for (key, value) in object
        .iter()
        .filter(|(key, _)| known.is_none_or(|props| !props.contains_key(*key)))
    {
        match schema.get("additionalProperties") {
            Some(Value::Bool(false)) => errors.push(format!("{path}.{key}: additional property")),
            Some(subschema) if subschema.is_object() => {
                crate::schema_catalog::schema::keywords::validate_at(
                    store,
                    root,
                    subschema,
                    value,
                    &join(path, key),
                    errors,
                    depth + 1,
                )
            }
            _ => {}
        }
    }
}

fn join(path: &str, key: &str) -> String {
    if path == "$" {
        format!("$.{key}")
    } else {
        format!("{path}.{key}")
    }
}
