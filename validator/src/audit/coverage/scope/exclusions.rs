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
    for pattern in value
        .pointer("/source_discovery_rules/ignore")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        let protected = pattern.strip_suffix("/**").unwrap_or(pattern);
        if repo_owned_path(protected) {
            out.push("coverage_repo_owned_code_ignored".to_string());
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
    ["src", "scripts", "validator", "schemas", "templates"]
        .iter()
        .any(|root| path == *root || path.starts_with(&format!("{root}/")))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn ignore_rules_cannot_hide_owned_source() {
        for pattern in ["validator/**", "scripts/check", "templates/**"] {
            let failures = super::failures(&json!({
                "source_discovery_rules": {"ignore": [pattern]}
            }));
            assert!(
                failures.contains(&"coverage_repo_owned_code_ignored".to_string()),
                "{pattern}: {failures:?}"
            );
        }
    }

    #[test]
    fn local_state_ignore_rules_remain_allowed() {
        let failures = super::failures(&json!({
            "source_discovery_rules": {"ignore": ["target/**", ".codex-worktree/**"]}
        }));
        assert!(
            !failures.contains(&"coverage_repo_owned_code_ignored".to_string()),
            "{failures:?}"
        );
    }
}
