use crate::audit::contract::Failure;
use serde_json::Value;

pub(crate) fn check(receipt: &Value, out: &mut Vec<Failure>) {
    for row in receipt
        .get("exclusions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        exclusion_row(row, out);
    }
}

fn exclusion_row(row: &Value, out: &mut Vec<Failure>) {
    for (bad, code) in [
        (
            repo_owned_path(&str_field(row, "path")),
            "coverage_repo_owned_code_excluded",
        ),
        (
            row.get("reviewed").and_then(Value::as_bool) != Some(true),
            "coverage_exclusion_unreviewed",
        ),
        (
            row.get("counts_as_covered").and_then(Value::as_bool) != Some(false),
            "coverage_exclusion_counted_as_covered",
        ),
        (
            str_field(row, "rationale").is_empty(),
            "coverage_exclusion_missing_rationale",
        ),
    ] {
        if bad {
            out.push(Failure::new(
                "coverage-proof-policy",
                code,
                str_field(row, "path"),
            ));
        }
    }
}

fn repo_owned_path(path: &str) -> bool {
    [
        "src/",
        "scripts/",
        "validator/",
        "schemas/",
        "skills/",
        ".codex/agents/",
        "connectors/",
        "templates/scripts/",
    ]
    .iter()
    .any(|prefix| path.starts_with(prefix))
}

fn str_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    #[test]
    fn only_current_agent_manifests_are_coverage_owned() {
        assert!(super::repo_owned_path(".codex/agents/claim-falsifier.toml"));
        assert!(!super::repo_owned_path(
            "custom-agents/harness-contract-claim-falsifier.toml"
        ));
        assert!(!super::repo_owned_path(
            "agents/contract-claim-falsifier.md"
        ));
    }
}
