use serde_json::Value;
use std::path::Path;

mod red;

pub(crate) const SOURCE_RECEIPT: &str = "validation_artifacts/observability/source-audit.json";
const TARGET_RECEIPT: &str = "validation_artifacts/observability/target-repo-audit.json";

pub(crate) fn write_all(
    root: &Path,
    audit_receipt: &Path,
    red_report: Option<&Path>,
    code: i32,
    target_repo: bool,
    command_error: Option<&str>,
) -> Result<(), String> {
    write_audit(root, audit_receipt, code, target_repo, command_error)?;
    if let Some(path) = red_report {
        red::write_report(root, path, command_error)?;
    }
    Ok(())
}

fn write_audit(
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
        TARGET_RECEIPT
    } else {
        SOURCE_RECEIPT
    };
    let audit = crate::json_boundary::read_json(receipt).unwrap_or(Value::Null);
    let failures = command_error
        .map(|err| vec![err.to_string()])
        .unwrap_or_else(|| failed_checks(&audit));
    let status = if code == 0 { "pass" } else { "fail" };
    let why_failed = if failures.is_empty() {
        if code == 0 {
            "none".to_string()
        } else {
            "source audit failed without check details".to_string()
        }
    } else {
        format!("source audit failed checks: {}", failures.join("; "))
    };
    emit_receipt(
        root,
        ReceiptFields {
            command: "ultragoal source",
            subcommand: "audit",
            operation,
            surface: if target_repo { "target_repo" } else { "source" },
            check_id: "source-audit-observability-binding",
            claim_id: if target_repo {
                "target_repo_audit"
            } else {
                "source_audit"
            },
            artifact_path: "validation_artifacts/ultragoal-audit",
            receipt_path: receipt_rel,
            status,
            failure_class: if code == 0 {
                "none"
            } else {
                "source_audit_check_failure"
            },
            why_failed: &why_failed,
            where_failed: if code == 0 { "none" } else { operation },
            next_repair: audit_next_repair(code),
            claim_impact: audit_claim_impact(code),
            supported_claims: audit_supported_claims(code),
        },
    )
}

pub(super) struct ReceiptFields<'a> {
    pub(super) command: &'a str,
    pub(super) subcommand: &'a str,
    pub(super) operation: &'a str,
    pub(super) surface: &'a str,
    pub(super) check_id: &'a str,
    pub(super) claim_id: &'a str,
    pub(super) artifact_path: &'a str,
    pub(super) receipt_path: &'a str,
    pub(super) status: &'a str,
    pub(super) failure_class: &'a str,
    pub(super) why_failed: &'a str,
    pub(super) where_failed: &'a str,
    pub(super) next_repair: &'a str,
    pub(super) claim_impact: &'a str,
    pub(super) supported_claims: Vec<String>,
}

pub(super) fn emit_receipt(root: &Path, fields: ReceiptFields<'_>) -> Result<(), String> {
    let value = crate::cli::observe::telemetry::command_receipt(
        root,
        crate::cli::observe::telemetry::CommandTelemetry {
            command: fields.command,
            subcommand: fields.subcommand,
            operation: fields.operation,
            surface: fields.surface,
            law_id: crate::cli::observe::types::LAW_ID,
            check_id: fields.check_id,
            claim_id: fields.claim_id,
            artifact_path: fields.artifact_path,
            receipt_path: fields.receipt_path,
            status: fields.status,
            failure_class: fields.failure_class,
            why_failed: fields.why_failed,
            where_failed: fields.where_failed,
            next_repair: fields.next_repair,
            claim_impact: fields.claim_impact,
            blocked_claims: blocked_claims(),
            supported_claims: fields.supported_claims,
            emit: true,
        },
    )?;
    crate::json_boundary::write_json(&root.join(fields.receipt_path), &value)?;
    println!(
        "ultragoal-audit-observe {} operation={} receipt={} run_id={} correlation_id={} claim_impact={}",
        fields.status,
        fields.operation,
        fields.receipt_path,
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

fn audit_next_repair(code: i32) -> &'static str {
    if code == 0 {
        "none"
    } else {
        "query this run through observe logs/metrics/traces, repair the named source-audit checks, then rerun source audit once"
    }
}

fn audit_claim_impact(code: i32) -> &'static str {
    if code == 0 {
        "supports_source_audit_pass_source_local_only"
    } else {
        "source_audit_failed_blocks_readiness_release_completion_update_goal"
    }
}

fn audit_supported_claims(code: i32) -> Vec<String> {
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
