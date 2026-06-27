use serde_json::Value;
use std::path::Path;

pub(crate) fn failures(root: &Path, value: &Value, pointer: &str, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, item) in map {
                let next = format!("{pointer}/{key}");
                if key == "evidence" {
                    evidence_ref_failure(root, item, &next, out);
                } else {
                    failures(root, item, &next, out);
                }
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                failures(root, item, &format!("{pointer}/{index}"), out);
            }
        }
        _ => {}
    }
}

fn evidence_ref_failure(root: &Path, value: &Value, pointer: &str, out: &mut Vec<String>) {
    let Some(obj) = value.as_object() else {
        out.push(format!("product_fitness_evidence_untyped:{pointer}"));
        return;
    };
    let path = obj.get("path").and_then(Value::as_str).unwrap_or("");
    let digest = obj.get("digest").and_then(Value::as_str).unwrap_or("");
    if path.is_empty() || digest.is_empty() {
        out.push(format!("product_fitness_evidence_untyped:{pointer}"));
        return;
    }
    if crate::package::inventory::package_path_error(root, path).is_some() {
        out.push(format!("product_fitness_evidence_path_invalid:{pointer}"));
        return;
    }
    match crate::digest::file(&root.join(path)) {
        Ok(actual) if actual == digest => {}
        Ok(_) => out.push(format!(
            "product_fitness_evidence_digest_mismatch:{pointer}"
        )),
        Err(_) => out.push(format!("product_fitness_evidence_missing:{pointer}")),
    }
}
