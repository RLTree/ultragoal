use serde_json::Value;
use std::path::Path;

pub(crate) fn failures(root: Option<&Path>, policy: &Value) -> Vec<String> {
    let Some(rows) = policy.get("changed_files").and_then(Value::as_array) else {
        return vec!["coverage_changed_file_missing_from_manifest".to_string()];
    };
    let Some(root) = root else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for row in rows {
        let Some(rel) = row.as_str() else {
            out.push("coverage_changed_file_missing_from_manifest".to_string());
            continue;
        };
        if crate::package::inventory::package_path_error(root, rel).is_some() {
            out.push("coverage_changed_file_missing_from_manifest".to_string());
            continue;
        }
        if rel == "validation_artifacts" || rel.starts_with("validation_artifacts/") {
            out.push("coverage_changed_file_generated_artifact".to_string());
            continue;
        }
        let path = root.join(rel);
        if !path.is_file() {
            out.push("coverage_changed_file_missing_from_manifest".to_string());
        }
    }
    out
}
