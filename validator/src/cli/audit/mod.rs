use serde_json::Value;
use std::path::{Path, PathBuf};

const SOURCE_AUDIT_OBSERVABILITY: &str = "validation_artifacts/observability/source-audit.json";

pub(crate) struct RunArgs {
    pub(crate) root: PathBuf,
    pub(crate) receipt: PathBuf,
    pub(crate) red_report: Option<PathBuf>,
    pub(crate) target_repo: Option<PathBuf>,
    pub(crate) mode: String,
    pub(crate) require_observability: bool,
    pub(crate) require_product_cohesion: bool,
}

pub(crate) fn run(args: RunArgs) -> Result<i32, String> {
    let root = args.root.clone();
    let receipt = args.receipt.clone();
    let target_repo = args.target_repo.clone();
    let options = crate::audit::AuditOptions {
        root: args.root,
        receipt: args.receipt,
        red_report: args.red_report,
        target_repo: args.target_repo,
        mode: args.mode,
        require_observability: args.require_observability,
        require_product_cohesion: args.require_product_cohesion,
        command_text: command_text(&root),
    };
    let result = crate::audit::run(options);
    match result {
        Ok(code) => {
            write_observability(&root, &receipt, code, target_repo.is_some(), None)?;
            Ok(code)
        }
        Err(err) => {
            let _ = write_observability(&root, &receipt, 1, target_repo.is_some(), Some(&err));
            Err(err)
        }
    }
}

fn write_observability(
    root: &Path,
    receipt: &Path,
    code: i32,
    target_repo: bool,
    command_error: Option<&str>,
) -> Result<(), String> {
    let operation = if target_repo {
        "target-repo.audit"
    } else {
        "source.audit"
    };
    let receipt_rel = if target_repo {
        "validation_artifacts/observability/target-repo-audit.json"
    } else {
        SOURCE_AUDIT_OBSERVABILITY
    };
    let audit = crate::json_boundary::read_json(receipt).unwrap_or(Value::Null);
    let failures = command_error
        .map(|err| vec![err.to_string()])
        .unwrap_or_else(|| failed_checks(&audit));
    let status = if code == 0 { "pass" } else { "fail" };
    let failure_class = if code == 0 {
        "none"
    } else {
        "source_audit_check_failure"
    };
    let why_failed = if failures.is_empty() {
        if code == 0 {
            "none".to_string()
        } else {
            "source audit failed without check details".to_string()
        }
    } else {
        format!("source audit failed checks: {}", failures.join("; "))
    };
    let value = crate::cli::observe::telemetry::command_receipt(
        root,
        crate::cli::observe::telemetry::CommandTelemetry {
            command: "ultragoal source",
            subcommand: "audit",
            operation,
            surface: if target_repo { "target_repo" } else { "source" },
            law_id: crate::cli::observe::types::LAW_ID,
            check_id: "source-audit-observability-binding",
            claim_id: if target_repo {
                "target_repo_audit"
            } else {
                "source_audit"
            },
            artifact_path: "validation_artifacts/ultragoal-audit",
            receipt_path: receipt_rel,
            status,
            failure_class,
            why_failed: &why_failed,
            where_failed: if code == 0 { "none" } else { operation },
            next_repair: next_repair(code),
            claim_impact: claim_impact(code),
            blocked_claims: blocked_claims(),
            supported_claims: supported_claims(code),
            emit: true,
        },
    )?;
    crate::json_boundary::write_json(&root.join(receipt_rel), &value)?;
    println!(
        "ultragoal-audit-observe {status} operation={operation} receipt={receipt_rel} run_id={} correlation_id={} claim_impact={}",
        value["run_id"].as_str().unwrap_or("<missing>"),
        value["correlation_id"].as_str().unwrap_or("<missing>"),
        value["claim_impact"].as_str().unwrap_or("<missing>")
    );
    Ok(())
}

fn failed_checks(audit: &Value) -> Vec<String> {
    let mut checks = audit
        .get("checks")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|rows| rows.iter())
        .filter_map(|(check, row)| {
            (row.get("status").and_then(Value::as_str) != Some("pass")).then(|| check.to_string())
        })
        .collect::<Vec<_>>();
    checks.sort();
    checks
}

fn next_repair(code: i32) -> &'static str {
    if code == 0 {
        "none"
    } else {
        "query this run through observe logs/metrics/traces, repair the named source-audit checks, then rerun source audit once"
    }
}

fn claim_impact(code: i32) -> &'static str {
    if code == 0 {
        "supports_source_audit_pass_source_local_only"
    } else {
        "source_audit_failed_blocks_readiness_release_completion_update_goal"
    }
}

fn supported_claims(code: i32) -> Vec<String> {
    if code == 0 {
        ["source_local_audit_checks", "red_fixture_report"]
            .into_iter()
            .map(ToString::to_string)
            .collect()
    } else {
        Vec::new()
    }
}

fn blocked_claims() -> Vec<String> {
    [
        "completion",
        "readiness",
        "release",
        "reviewer_exposure",
        "app_registry_exposure",
        "final_packet_correctness",
        "update_goal_eligibility",
    ]
    .into_iter()
    .map(ToString::to_string)
    .collect()
}

fn command_text(root: &Path) -> String {
    let raw = std::env::args().collect::<Vec<_>>().join(" ");
    raw.replace(&root.to_string_lossy().to_string(), ".")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;

    #[test]
    fn source_audit_observability_receipt_blocks_claims_on_failed_audit() {
        let root = crate::self_tests::boundaries::support::temp_root("source-audit-observe");
        fs::create_dir_all(&root).expect("root");
        fs::write(root.join("owned.txt"), "owned").expect("owned");
        crate::json_boundary::write_json(
            &root.join("plugin-manifest-draft.json"),
            &json!({"resources":["owned.txt"]}),
        )
        .expect("manifest");
        let receipt = root.join("validation_artifacts/ultragoal-audit/validator-receipt.json");
        fs::create_dir_all(receipt.parent().unwrap()).expect("audit dir");
        crate::json_boundary::write_json(
            &receipt,
            &json!({
                "status":"fail",
                "checks": {
                    "coverage-receipt": {"status":"fail"},
                    "schema-valid": {"status":"pass"}
                }
            }),
        )
        .expect("audit receipt");
        write_observability(&root, &receipt, 1, false, None).expect("observability");
        let value =
            crate::json_boundary::read_json(&root.join(SOURCE_AUDIT_OBSERVABILITY)).expect("obs");
        assert_eq!(value["status"], "fail");
        assert_eq!(value["operation"], "source.audit");
        assert_eq!(value["claim_id"], "source_audit");
        assert!(
            value["why_failed"]
                .as_str()
                .unwrap_or_default()
                .contains("coverage-receipt")
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
