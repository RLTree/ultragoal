use super::{ReceiptFields, emit_receipt};
use serde_json::Value;
use std::path::Path;

const RECEIPT: &str = "validation_artifacts/observability/red-fixture-report.json";

pub(super) fn write_report(
    root: &Path,
    red_report: &Path,
    command_error: Option<&str>,
) -> Result<(), String> {
    let report = crate::json_boundary::read_json(red_report).unwrap_or(Value::Null);
    let status = if report.get("status").and_then(Value::as_str) == Some("pass") {
        "pass"
    } else {
        "fail"
    };
    let failures = failures(&report);
    let why_failed = failure_reason(status, command_error, &failures);
    emit_receipt(
        root,
        ReceiptFields {
            command: "ultragoal source",
            subcommand: "audit --red-report",
            operation: "red_fixture.report",
            surface: "source",
            check_id: "red-fixture-report-observability-binding",
            claim_id: "red_fixture_report",
            artifact_path: "validation_artifacts/ultragoal-audit/red-fixture-report.json",
            receipt_path: RECEIPT,
            status,
            failure_class: if status == "pass" {
                "none"
            } else {
                "red_fixture_report_failure"
            },
            why_failed: &why_failed,
            where_failed: if status == "pass" {
                "none"
            } else {
                "red_fixture.report"
            },
            next_repair: next_repair(status),
            claim_impact: claim_impact(status),
            supported_claims: supported_claims(status),
        },
    )
}

fn failure_reason(status: &str, command_error: Option<&str>, failures: &[String]) -> String {
    if status == "pass" {
        "none".to_string()
    } else if let Some(err) = command_error {
        err.to_string()
    } else if failures.is_empty() {
        "red fixture report missing, malformed, stale, or not pass".to_string()
    } else {
        format!(
            "red fixture report failed fixtures: {}",
            failures.join("; ")
        )
    }
}

fn failures(report: &Value) -> Vec<String> {
    let mut out = report
        .get("red_fixtures")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|rows| rows.iter())
        .filter_map(|(fixture, row)| {
            (row.get("status").and_then(Value::as_str) != Some("pass")).then(|| fixture.to_string())
        })
        .collect::<Vec<_>>();
    out.sort();
    out
}

fn next_repair(status: &str) -> &'static str {
    if status == "pass" {
        "none"
    } else {
        "query this run through observe logs/metrics/traces, repair failing red fixtures, then rerun source audit with --red-report once"
    }
}

fn claim_impact(status: &str) -> &'static str {
    if status == "pass" {
        "supports_red_fixture_report_source_local_only"
    } else {
        "red_fixture_report_failed_blocks_readiness_release_completion_update_goal"
    }
}

fn supported_claims(status: &str) -> Vec<String> {
    if status == "pass" {
        vec!["red_fixture_report".to_string()]
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;

    #[test]
    fn red_fixture_report_observability_receipt_supports_only_red_report() {
        let root = crate::self_tests::boundaries::support::temp_root("red-report-observe");
        fs::create_dir_all(root.join("validation_artifacts/ultragoal-audit")).expect("audit dir");
        crate::json_boundary::write_json(
            &root.join("plugin-manifest-draft.json"),
            &json!({"resources":[]}),
        )
        .expect("manifest");
        let report = root.join("validation_artifacts/ultragoal-audit/red-fixture-report.json");
        crate::json_boundary::write_json(
            &report,
            &json!({"status":"pass","red_fixtures":{"red-one":{"status":"pass"}}}),
        )
        .expect("red report");
        write_report(&root, &report, None).expect("observability");
        let value = crate::json_boundary::read_json(&root.join(RECEIPT)).expect("red receipt");
        assert_eq!(value["status"], "pass");
        assert_eq!(value["operation"], "red_fixture.report");
        assert_eq!(value["claim_id"], "red_fixture_report");
        assert!(
            value["supported_claims"]
                .as_array()
                .expect("supported")
                .iter()
                .any(|item| item.as_str() == Some("red_fixture_report"))
        );
        assert!(
            value["blocked_claims"]
                .as_array()
                .expect("blocked")
                .iter()
                .any(|item| item.as_str() == Some("update_goal_eligibility"))
        );
        fs::remove_dir_all(root).expect("cleanup");
    }
}
