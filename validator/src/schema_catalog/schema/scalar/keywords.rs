use serde_json::Value;

pub(crate) fn check(schema: &Value, instance: &Value, path: &str, errors: &mut Vec<String>) {
    string_checks(schema, instance, path, errors);
    number_checks(schema, instance, path, errors);
}

pub(crate) fn number_checks(
    schema: &Value,
    instance: &Value,
    path: &str,
    errors: &mut Vec<String>,
) {
    let Some(number) = instance.as_f64() else {
        return;
    };
    if let Some(min) = schema.get("minimum").and_then(Value::as_f64)
        && number < min
    {
        errors.push(format!("{path}: minimum"));
    }
    if let Some(max) = schema.get("maximum").and_then(Value::as_f64)
        && number > max
    {
        errors.push(format!("{path}: maximum"));
    }
}

fn string_checks(schema: &Value, instance: &Value, path: &str, errors: &mut Vec<String>) {
    let Some(text) = instance.as_str() else {
        return;
    };
    if let Some(min) = schema.get("minLength").and_then(Value::as_u64)
        && text.chars().count() < min as usize
    {
        errors.push(format!("{path}: minLength"));
    }
    if let Some(pattern) = schema.get("pattern").and_then(Value::as_str)
        && !crate::schema_catalog::schema::patterns::matches(pattern, text)
    {
        errors.push(format!("{path}: pattern mismatch"));
    }
    if schema.get("format").and_then(Value::as_str) == Some("date-time")
        && crate::audit::clock::parse_iso_seconds(text).is_none()
    {
        errors.push(format!("{path}: format date-time"));
    }
}
