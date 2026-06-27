use serde_json::Value;
use std::path::Path;

const RED_REPORT: &str = "validation_artifacts/ultragoal-audit/red-fixture-report.json";

pub(super) fn report_failures(
    root: &Path,
    store: &crate::schema_catalog::SchemaStore,
) -> Vec<String> {
    let mut out = Vec::new();
    let Some(receipt) = read_json(root, RED_REPORT, &mut out) else {
        return out;
    };
    out.extend(
        crate::schema_catalog::schema_errors(store, "red-fixture-report.schema.json", &receipt)
            .into_iter()
            .map(|failure| format!("red_fixture_report_schema:{failure}")),
    );
    if receipt.get("status").and_then(Value::as_str) != Some("pass") {
        out.push("red_fixture_report_status_not_pass".to_string());
    }
    let failing = receipt
        .get("red_fixtures")
        .and_then(Value::as_object)
        .map(|rows| {
            rows.values()
                .filter(|row| row.get("status").and_then(Value::as_str) != Some("pass"))
                .count()
        })
        .unwrap_or(usize::MAX);
    if failing != 0 {
        out.push(format!("red_fixture_report_has_failing_rows:{failing}"));
    }
    out
}

fn read_json(root: &Path, rel: &str, out: &mut Vec<String>) -> Option<Value> {
    match crate::json_boundary::read_json(&root.join(rel)) {
        Ok(value) => Some(value),
        Err(err) => {
            out.push(format!("{rel}:json_missing_or_malformed:{err}"));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("validator has repo parent")
            .to_path_buf()
    }

    fn temp_root(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("{name}-{stamp}"))
    }

    fn write_json(path: &Path, value: &Value) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("parent");
        }
        std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
    }

    #[test]
    fn red_report_failures_report_nonpassing_rows_without_schema_theater() {
        let root = temp_root("cli-red-report-failures");
        write_json(
            &root.join(RED_REPORT),
            &json!({
                "schema":"harness-ultragoal.red-fixture-report.v1",
                "status":"fail",
                "generated_at":"2026-06-27T00:00:00Z",
                "red_fixtures":{
                    "row":{"status":"fail","packet_path":"fixtures/red/row.json","packet_digest":crate::digest::ZERO}
                }
            }),
        );
        let store = crate::schema_catalog::load(&repo_root());
        let failures = report_failures(&root, &store);
        assert!(
            failures
                .iter()
                .any(|failure| failure == "red_fixture_report_status_not_pass"),
            "{failures:?}"
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure == "red_fixture_report_has_failing_rows:1"),
            "{failures:?}"
        );
        assert!(
            failures
                .iter()
                .all(|failure| !failure.contains("red_fixture_report_schema")),
            "{failures:?}"
        );
        std::fs::remove_dir_all(root).expect("cleanup red report failures");
    }

    #[test]
    fn red_report_failures_reject_top_level_fail_even_when_rows_pass() {
        let root = temp_root("cli-red-report-status-fail");
        write_json(
            &root.join(RED_REPORT),
            &json!({
                "schema":"harness-ultragoal.red-fixture-report.v1",
                "status":"fail",
                "generated_at":"2026-06-27T00:00:00Z",
                "red_fixtures":{
                    "row":{"status":"pass","packet_path":"fixtures/red/row.json","packet_digest":crate::digest::ZERO}
                }
            }),
        );
        let store = crate::schema_catalog::load(&repo_root());
        let failures = report_failures(&root, &store);
        assert_eq!(failures, vec!["red_fixture_report_status_not_pass"]);
        std::fs::remove_dir_all(root).expect("cleanup red report status");
    }

    #[test]
    fn red_report_failures_bind_schema_errors_to_control_plane() {
        let root = temp_root("cli-red-report-schema-fail");
        write_json(
            &root.join(RED_REPORT),
            &json!({
                "schema":"harness-ultragoal.red-fixture-report.v1",
                "status":"pass",
                "generated_at":"2026-06-27T00:00:00Z",
                "red_fixtures":{
                    "row":{"status":"pass","packet_path":"../escape.json","packet_digest":crate::digest::ZERO}
                }
            }),
        );
        let store = crate::schema_catalog::load(&repo_root());
        let failures = report_failures(&root, &store);
        assert!(
            failures
                .iter()
                .any(|failure| failure.starts_with("red_fixture_report_schema:")),
            "{failures:?}"
        );
        assert!(!failures.contains(&"red_fixture_report_status_not_pass".to_string()));
        std::fs::remove_dir_all(root).expect("cleanup red report schema");
    }
}
