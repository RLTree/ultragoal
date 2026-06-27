use crate::cli::control::plane::types::ControlOperation;
use serde_json::Value;
use std::path::Path;

const RED_REPORT: &str = "validation_artifacts/ultragoal-audit/red-fixture-report.json";

pub(crate) fn failures(root: &Path, _operation: ControlOperation) -> Vec<String> {
    let store = crate::schema_catalog::load(root);
    let mut out = store
        .errors
        .iter()
        .map(|failure| format!("schema_catalog:{failure}"))
        .collect::<Vec<_>>();
    out.extend(prefixed(
        "plugin_self_law",
        crate::audit::plugin::laws::package_failures(root, &store),
    ));
    out.extend(prefixed(
        "cli_performance",
        crate::audit::cli::performance::package_failures(root),
    ));
    out.extend(prefixed(
        "final_packet",
        crate::audit::final_packet::package_failures(root, &store),
    ));
    out.extend(red_report_failures(root, &store));
    out
}

pub(crate) fn failure_value(operation: ControlOperation, failures: &[String]) -> Value {
    if failures.is_empty() {
        return Value::Null;
    }
    let law = failure_law(operation);
    serde_json::json!({
        "id": format!("{}_evidence_not_satisfied", operation.id()),
        "law_id": law,
        "gate_id": "89",
        "check_id": law,
        "failed_invariant": "CLI control-plane claims require current same-candidate typed evidence",
        "observed_value": failures.join(" | "),
        "expected_value": "all_required_cli_authority_evidence_passes_for_current_candidate",
        "repair_class": "deterministic_enforcement",
        "rerun_command": "ultragoal update-goal eligibility --receipt validation_artifacts/cli/update-goal-eligibility.json",
        "claim_ceiling_impact": "completion_package_review_release_update_goal_withheld",
        "source_install_cache_impact": "source_only_proof_cannot_support_install_cache_app_registry_claims",
        "severity": "hard_blocker",
        "determinism": "deterministic"
    })
}

pub(crate) fn notes(pass: bool, failures: &[String]) -> String {
    if pass {
        "CLI authority evidence is current, self-hosted, and same-candidate.".to_string()
    } else {
        format!(
            "Fail-closed CLI authority receipt. Evidence failures: {}",
            failures.join("; ")
        )
    }
}

fn red_report_failures(root: &Path, store: &crate::schema_catalog::SchemaStore) -> Vec<String> {
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

fn prefixed(prefix: &str, failures: Vec<String>) -> Vec<String> {
    failures
        .into_iter()
        .map(|failure| format!("{prefix}:{failure}"))
        .collect()
}

fn failure_law(operation: ControlOperation) -> &'static str {
    if matches!(operation, ControlOperation::SelfUpdateGoalEligibility) {
        "cli-self-law-compliance"
    } else {
        "cli-control-plane-authority"
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
    fn proof_helpers_return_null_failure_and_pass_notes_for_empty_evidence() {
        let failures = Vec::new();
        assert_eq!(
            failure_value(ControlOperation::UpdateGoalEligibility, &failures),
            Value::Null
        );
        assert!(notes(true, &failures).contains("self-hosted"));
        assert_eq!(
            failure_law(ControlOperation::SelfUpdateGoalEligibility),
            "cli-self-law-compliance"
        );
        assert_eq!(
            failure_law(ControlOperation::PacketVerify),
            "cli-control-plane-authority"
        );
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
        let failures = red_report_failures(&root, &store);
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
        let failures = red_report_failures(&root, &store);
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
        let failures = red_report_failures(&root, &store);
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
