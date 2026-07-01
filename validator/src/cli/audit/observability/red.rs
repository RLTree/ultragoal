use super::{ReceiptFields, RuntimeFacts, emit_receipt};
use serde_json::Value;
use std::path::Path;

const RECEIPT: &str = "validation_artifacts/observability/red-fixture-report.json";

pub(super) fn write_report(
    root: &Path,
    red_report: &Path,
    command_error: Option<&str>,
    runtime: RuntimeFacts,
) -> Result<(), String> {
    write_with_context(
        root,
        red_report,
        command_error,
        runtime,
        ReportContext::source_audit(),
    )
    .map(|_| ())
}

pub(super) fn write_standalone(
    root: &Path,
    red_report: &Path,
    runtime: RuntimeFacts,
) -> Result<i32, String> {
    write_with_context(root, red_report, None, runtime, ReportContext::standalone())
        .map(red_report_exit_code)
}

fn write_with_context(
    root: &Path,
    red_report: &Path,
    command_error: Option<&str>,
    runtime: RuntimeFacts,
    context: ReportContext<'_>,
) -> Result<Value, String> {
    let current = crate::package::inventory::package_digest(root)?;
    let report = crate::json_boundary::read_json(red_report).unwrap_or(Value::Null);
    let stale = stale_reason(&report, &current);
    let status = if report.get("status").and_then(Value::as_str) == Some("pass") && stale.is_none()
    {
        "pass"
    } else {
        "fail"
    };
    let failures = failures(&report);
    let why_failed = failure_reason(status, command_error, stale.as_deref(), &failures);
    emit_receipt(
        root,
        ReceiptFields {
            command: context.command,
            subcommand: context.subcommand,
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
            next_repair: context.next_repair(status),
            claim_impact: claim_impact(status),
            supported_claims: supported_claims(status),
            runtime: crate::cli::observe::telemetry::RuntimeTelemetry {
                duration_ms: runtime.elapsed_ms(),
                worker_count: 0,
                task_count: 0,
                queue_depth: 0,
                cpu_ms: None,
                memory_bytes: None,
                io_bytes: None,
                cache_mode: context.cache_mode.to_string(),
                resource_measurement_status: context.resource_status.to_string(),
                retry_count: 0,
                backoff_ms: 0,
                saturation_status: context.saturation_status.to_string(),
                repair_anchor_before: context.repair_anchor_before.to_string(),
                repair_anchor_after: "red_fixture_report_observability_emit".to_string(),
            },
        },
    )
}

struct ReportContext<'a> {
    command: &'a str,
    subcommand: &'a str,
    cache_mode: &'a str,
    resource_status: &'a str,
    saturation_status: &'a str,
    repair_anchor_before: &'a str,
    next_repair_failure: &'a str,
}

impl<'a> ReportContext<'a> {
    fn source_audit() -> Self {
        Self {
            command: "ultragoal source",
            subcommand: "audit --red-report",
            cache_mode: "red_report_read_after_source_audit",
            resource_status: "source_audit_runtime_wall_time_only",
            saturation_status: "red_report_no_scheduler_tasks_started",
            repair_anchor_before: "source_audit_red_report_requested",
            next_repair_failure: "query this run through observe logs/metrics/traces, repair failing red fixtures, then rerun source audit with --red-report once",
        }
    }

    fn standalone() -> Self {
        Self {
            command: "ultragoal red",
            subcommand: "fixture report",
            cache_mode: "red_report_standalone_read",
            resource_status: "standalone_red_report_wall_time_only",
            saturation_status: "standalone_red_report_no_scheduler_tasks_started",
            repair_anchor_before: "red_fixture_report_command_start",
            next_repair_failure: "query this run through observe logs/metrics/traces, repair failing red fixtures or regenerate the red report, then rerun red fixture report",
        }
    }

    fn next_repair(&self, status: &str) -> &'a str {
        if status == "pass" {
            "none"
        } else {
            self.next_repair_failure
        }
    }
}

fn stale_reason(report: &Value, current: &str) -> Option<String> {
    if report.get("status").and_then(Value::as_str) != Some("pass") {
        return None;
    }
    let observed = report_candidate(report);
    (observed != Some(current)).then(|| {
        format!(
            "red fixture report target revision {} does not match current candidate {current}",
            observed.unwrap_or("<missing>")
        )
    })
}

fn report_candidate(report: &Value) -> Option<&str> {
    report
        .pointer("/target_revision/value")
        .and_then(Value::as_str)
        .or_else(|| report.get("target_package_digest").and_then(Value::as_str))
        .or_else(|| report.get("package_digest").and_then(Value::as_str))
}

fn failure_reason(
    status: &str,
    command_error: Option<&str>,
    stale: Option<&str>,
    failures: &[String],
) -> String {
    if status == "pass" {
        "none".to_string()
    } else if let Some(err) = command_error {
        err.to_string()
    } else if let Some(reason) = stale {
        reason.to_string()
    } else if failures.is_empty() {
        "red fixture report missing, malformed, stale, or not pass".to_string()
    } else {
        format!(
            "red fixture report failed fixtures: {}",
            failures.join("; ")
        )
    }
}

fn red_report_exit_code(value: Value) -> i32 {
    i32::from(value.get("status").and_then(Value::as_str) != Some("pass"))
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
