use serde_json::Value;
use std::path::Path;

const REQUIRED_COMMANDS: &[&str] = &[
    "observe stack up",
    "observe stack health",
    "observe stack smoke",
    "observe stack down",
    "observe stack gc plan",
    "observe stack gc dry-run",
    "observe stack gc apply",
    "observe logs query",
    "observe metrics query",
    "observe traces query",
    "observe snapshot",
    "observe prove",
    "observe explain-failure",
    "observe explain-claim",
    "observe explain-check",
    "observe explain-law",
];

pub(super) fn check(root: &Path, out: &mut Vec<String>) {
    require_law_rows(root, out);
    require_command_inventory(root, out);
}

fn require_law_rows(root: &Path, out: &mut Vec<String>) {
    for (rel, key, code) in [
        (
            "templates/agent-standards/enforcement.json",
            "rows",
            "missing_standards_row",
        ),
        (
            "docs/source-obligation-matrix.json",
            "obligations",
            "missing_source_obligation",
        ),
        (
            "docs/foundational-law-traceability.json",
            "entries",
            "missing_foundational_trace",
        ),
    ] {
        if !has_law_id(root, rel, key) {
            out.push(format!("observability_{code}:{}", super::LAW));
        }
    }
    let valid = format!("fixtures/mandatory-law-surfaces/valid/{}.json", super::LAW);
    if !root.join(&valid).is_file() {
        out.push(format!("observability_missing_valid_fixture:{valid}"));
    }
}

fn require_command_inventory(root: &Path, out: &mut Vec<String>) {
    let value = super::read::json(root, "docs/generated/observability/command-inventory.json");
    for command in REQUIRED_COMMANDS {
        if !commands_contain(&value, command) {
            out.push(format!("observability_command_inventory_missing:{command}"));
        }
    }
    for key in row_requirement_keys() {
        if value.pointer(&format!("/row_requirements/{key}")) != Some(&Value::Bool(true)) {
            out.push(format!(
                "observability_command_inventory_requirement_missing:{key}"
            ));
        }
    }
}

fn commands_contain(value: &Value, command: &str) -> bool {
    value
        .get("commands")
        .and_then(Value::as_array)
        .is_some_and(|rows| rows.iter().any(|row| row.as_str() == Some(command)))
}

fn has_law_id(root: &Path, rel: &str, key: &str) -> bool {
    super::read::json(root, rel)
        .get(key)
        .and_then(Value::as_array)
        .is_some_and(|rows| {
            rows.iter().any(|row| {
                row.get("id")
                    .or_else(|| row.get("obligation_id"))
                    .and_then(Value::as_str)
                    == Some(super::LAW)
            })
        })
}

fn row_requirement_keys() -> [&'static str; 8] {
    [
        "log_instrumentation",
        "metric_instrumentation",
        "trace_instrumentation",
        "pass_output_contract",
        "fail_output_contract",
        "receipt_observability_binding",
        "focused_tests",
        "claim_impact_mapping",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;

    #[test]
    fn observability_registry_reports_missing_valid_fixture_and_accepts_obligation_id() {
        let root = crate::self_tests::boundaries::support::temp_root("observe-registry");
        fs::create_dir_all(root.join("templates/agent-standards")).expect("standards");
        fs::create_dir_all(root.join("docs/generated/observability")).expect("inventory");
        fs::create_dir_all(root.join("docs")).expect("docs");
        fs::create_dir_all(root.join("fixtures/mandatory-law-surfaces/valid")).expect("fixtures");
        crate::json_boundary::write_json(
            &root.join("templates/agent-standards/enforcement.json"),
            &json!({"rows":[{"id": super::super::LAW}]}),
        )
        .expect("standards json");
        crate::json_boundary::write_json(
            &root.join("docs/source-obligation-matrix.json"),
            &json!({"obligations":[{"obligation_id": super::super::LAW}]}),
        )
        .expect("obligation json");
        crate::json_boundary::write_json(
            &root.join("docs/foundational-law-traceability.json"),
            &json!({"entries":[{"id": super::super::LAW}]}),
        )
        .expect("trace json");
        crate::json_boundary::write_json(
            &root.join("docs/generated/observability/command-inventory.json"),
            &json!({"commands": REQUIRED_COMMANDS, "row_requirements": {
                "log_instrumentation": true,
                "metric_instrumentation": true,
                "trace_instrumentation": true,
                "pass_output_contract": true,
                "fail_output_contract": true,
                "receipt_observability_binding": true,
                "focused_tests": true,
                "claim_impact_mapping": true
            }}),
        )
        .expect("inventory json");
        let mut failures = Vec::new();
        check(&root, &mut failures);
        assert_eq!(
            failures,
            vec![format!(
                "observability_missing_valid_fixture:fixtures/mandatory-law-surfaces/valid/{}.json",
                super::super::LAW
            )]
        );
        fs::write(
            root.join(format!(
                "fixtures/mandatory-law-surfaces/valid/{}.json",
                super::super::LAW
            )),
            "{}",
        )
        .expect("valid fixture");
        failures.clear();
        check(&root, &mut failures);
        assert!(failures.is_empty(), "{failures:?}");
        fs::remove_dir_all(root).expect("cleanup registry");
    }
}
