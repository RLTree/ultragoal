use serde_json::Value;

pub fn apply_patch(doc: &Value, ops: &Value) -> Result<Value, String> {
    let mut out = doc.clone();
    let Some(items) = ops.as_array() else {
        return Err("json_patch must be an array".to_string());
    };
    for op in items {
        let action = op.get("op").and_then(Value::as_str).unwrap_or("");
        let path = op.get("path").and_then(Value::as_str).unwrap_or("");
        let parts = pointer_parts(path);
        apply_one(&mut out, &parts, action, op.get("value").cloned())?;
    }
    Ok(out)
}

fn pointer_parts(pointer: &str) -> Vec<String> {
    if pointer.is_empty() {
        return Vec::new();
    }
    pointer
        .trim_start_matches('/')
        .split('/')
        .map(|part| part.replace("~1", "/").replace("~0", "~"))
        .collect()
}

fn apply_one(
    node: &mut Value,
    parts: &[String],
    action: &str,
    value: Option<Value>,
) -> Result<(), String> {
    if parts.is_empty() {
        return Err("json pointer is empty".to_string());
    }
    let mut cur = node;
    for part in &parts[..parts.len() - 1] {
        cur = match cur {
            Value::Array(array) => {
                let index = part
                    .parse::<usize>()
                    .map_err(|_| format!("invalid array index {part}"))?;
                array
                    .get_mut(index)
                    .ok_or_else(|| format!("array index missing {part}"))?
            }
            Value::Object(object) => object
                .get_mut(part)
                .ok_or_else(|| format!("object key missing {part}"))?,
            _ => return Err("json pointer parent is not object or array".to_string()),
        };
    }
    let key = parts.last().ok_or("json pointer is empty")?;
    if let Some(array) = cur.as_array_mut() {
        apply_array(array, key, action, value)
    } else {
        let object = cur
            .as_object_mut()
            .ok_or_else(|| "json pointer parent is not object or array".to_string())?;
        match action {
            "add" | "replace" => {
                object.insert(key.clone(), value.unwrap_or(Value::Null));
                Ok(())
            }
            "remove" => {
                object.remove(key);
                Ok(())
            }
            _ => Err(format!("unsupported patch op {action}")),
        }
    }
}

fn apply_array(
    array: &mut Vec<Value>,
    key: &str,
    action: &str,
    value: Option<Value>,
) -> Result<(), String> {
    let index = if key == "-" {
        array.len()
    } else {
        key.parse::<usize>()
            .map_err(|_| format!("invalid array index {key}"))?
    };
    match action {
        "add" => {
            if index > array.len() {
                return Err(format!("array index missing {key}"));
            }
            array.insert(index, value.unwrap_or(Value::Null));
            Ok(())
        }
        "replace" => {
            let slot = array
                .get_mut(index)
                .ok_or_else(|| format!("array index missing {key}"))?;
            *slot = value.unwrap_or(Value::Null);
            Ok(())
        }
        "remove" => {
            if index >= array.len() {
                return Err(format!("array index missing {key}"));
            }
            array.remove(index);
            Ok(())
        }
        _ => Err(format!("unsupported patch op {action}")),
    }
}
