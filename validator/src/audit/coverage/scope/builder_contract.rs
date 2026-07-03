use serde_json::Value;

pub(super) fn manifest_dependency_failures(value: &Value) -> Vec<String> {
    let mut out = Vec::new();
    for rel in strings(value, "required_target_paths") {
        push_if_parent_contract(&rel, &mut out);
    }
    for rel in strings(
        value.get("source_discovery_rules").unwrap_or(&Value::Null),
        "ignore",
    ) {
        push_if_parent_contract(&rel, &mut out);
    }
    for rel in strings(
        value
            .get("changed_file_coupling_policy")
            .unwrap_or(&Value::Null),
        "changed_files",
    ) {
        push_if_parent_contract(&rel, &mut out);
    }
    out
}

fn push_if_parent_contract(rel: &str, out: &mut Vec<String>) {
    if crate::package::inventory::builder_contract_resource_path(rel) {
        out.push("coverage_builder_contract_resource_dependency".to_string());
    }
}

fn strings(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect()
}
