use serde_json::Value;

const ALLOWED: &[&str] = &[
    "$defs",
    "$id",
    "$ref",
    "$schema",
    "additionalProperties",
    "allOf",
    "anyOf",
    "const",
    "contains",
    "default",
    "description",
    "else",
    "enum",
    "format",
    "if",
    "items",
    "maxContains",
    "maxItems",
    "maxProperties",
    "maximum",
    "minContains",
    "minItems",
    "minLength",
    "minProperties",
    "minimum",
    "not",
    "oneOf",
    "pattern",
    "properties",
    "propertyNames",
    "required",
    "then",
    "title",
    "type",
    "uniqueItems",
];

pub fn errors(name: &str, schema: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    visit_schema(name, "$", schema, &mut errors);
    errors
}

fn visit_schema(name: &str, path: &str, schema: &Value, errors: &mut Vec<String>) {
    let Some(object) = schema.as_object() else {
        return;
    };
    for (key, value) in object {
        if !ALLOWED.contains(&key.as_str()) {
            errors.push(format!("{name}: unsupported schema keyword {path}.{key}"));
            continue;
        }
        visit_keyword(name, path, key, value, errors);
    }
}

fn visit_keyword(name: &str, path: &str, key: &str, value: &Value, errors: &mut Vec<String>) {
    match key {
        "$defs" | "properties" => visit_named_schema_map(name, path, key, value, errors),
        "additionalProperties"
        | "contains"
        | "items"
        | "propertyNames"
        | "if"
        | "then"
        | "else"
        | "not" => visit_child_schema(name, path, key, value, errors),
        "allOf" | "anyOf" | "oneOf" => visit_schema_array(name, path, key, value, errors),
        _ => {}
    }
}

fn visit_named_schema_map(
    name: &str,
    path: &str,
    key: &str,
    value: &Value,
    errors: &mut Vec<String>,
) {
    let Some(map) = value.as_object() else {
        return;
    };
    for (field, child) in map {
        visit_schema(name, &format!("{path}.{key}.{field}"), child, errors);
    }
}

fn visit_child_schema(name: &str, path: &str, key: &str, value: &Value, errors: &mut Vec<String>) {
    if value.is_object() || value.is_boolean() {
        visit_schema(name, &format!("{path}.{key}"), value, errors);
    }
}

fn visit_schema_array(name: &str, path: &str, key: &str, value: &Value, errors: &mut Vec<String>) {
    let Some(items) = value.as_array() else {
        return;
    };
    for (index, child) in items.iter().enumerate() {
        visit_schema(name, &format!("{path}.{key}[{index}]"), child, errors);
    }
}
