use serde_json::Value;

pub(crate) fn failures(value: &Value) -> Vec<String> {
    let mut out = Vec::new();
    for row in value
        .get("exclusions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if str_field(row, "rationale").is_empty() {
            out.push("coverage_exclusion_missing_rationale".to_string());
        }
        if row.get("reviewed").and_then(Value::as_bool) != Some(true) {
            out.push("coverage_exclusion_unreviewed".to_string());
        }
        if row.get("counts_as_covered").and_then(Value::as_bool) != Some(false) {
            out.push("coverage_exclusion_counted_as_covered".to_string());
        }
        if repo_owned_path(&str_field(row, "path")) {
            out.push("coverage_repo_owned_code_excluded".to_string());
        }
    }
    out
}

fn str_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn repo_owned_path(path: &str) -> bool {
    ["src/", "scripts/", "validator/", "schemas/", "templates/"]
        .iter()
        .any(|prefix| path.starts_with(prefix))
}
