use serde_json::Value;
use std::path::Path;

mod red;
#[cfg(test)]
mod red_tests;
mod runtime;
#[cfg(test)]
mod runtime_tests;
mod stdout;

pub(crate) const SOURCE_RECEIPT: &str = "validation_artifacts/observability/source-audit.json";
const TARGET_RECEIPT: &str = "validation_artifacts/observability/target-repo-audit.json";

pub(crate) use runtime::RuntimeFacts;

pub(crate) fn write_all(
    root: &Path,
    audit_receipt: &Path,
    red_report: Option<&Path>,
    code: i32,
    target_repo: bool,
    command_error: Option<&str>,
    runtime: RuntimeFacts,
) -> Result<(), String> {
    write_audit(
        root,
        audit_receipt,
        code,
        target_repo,
        command_error,
        runtime,
    )?;
    if let Some(path) = red_report {
        red::write_report(root, path, command_error, runtime)?;
    }
    Ok(())
}

pub(crate) fn write_standalone_red_report(
    root: &Path,
    red_report: &Path,
    runtime: RuntimeFacts,
) -> Result<i32, String> {
    red::write_standalone(root, red_report, runtime)
}

fn write_audit(
    root: &Path,
    receipt: &Path,
    code: i32,
    target_repo: bool,
    command_error: Option<&str>,
    runtime: RuntimeFacts,
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
    let runtime = runtime::telemetry(&audit, runtime);
    let status = if code == 0 { "pass" } else { "fail" };
    let why_failed = stdout::audit_failure_summary(&audit, command_error, code);
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
            runtime,
        },
    )
    .map(|_| ())
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
    pub(super) runtime: crate::cli::observe::telemetry::RuntimeTelemetry,
}

pub(super) fn emit_receipt(root: &Path, fields: ReceiptFields<'_>) -> Result<Value, String> {
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
            runtime: Some(fields.runtime),
            emit: true,
        },
    )?;
    let receipt = crate::output_path::claim_artifact_path(
        root,
        Path::new(fields.receipt_path),
        "audit observability receipt",
    )?;
    crate::json_boundary::write_json(&receipt, &value)?;
    for line in stdout::contract(&value) {
        println!("{line}");
    }
    Ok(value)
}

#[cfg(test)]
pub(crate) fn stdout_contract_for_test(value: &Value) -> Vec<String> {
    stdout::contract(value)
}

fn audit_next_repair(code: i32) -> &'static str {
    if code == 0 {
        "none"
    } else {
        "query this run through observe logs/metrics/traces, repair the first_check/root_group named in why_failed, then rerun source audit once"
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
        vec![
            "source_local_audit_checks".to_string(),
            "red_fixture_report".to_string(),
        ]
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
