use std::path::{Path, PathBuf};

#[cfg(test)]
mod edge_tests;
mod observability;
#[cfg(test)]
mod parse_tests;
#[cfg(test)]
mod tests;

pub(crate) struct RunArgs {
    pub(crate) root: PathBuf,
    pub(crate) receipt: PathBuf,
    pub(crate) red_report: Option<PathBuf>,
    pub(crate) target_repo: Option<PathBuf>,
    pub(crate) mode: String,
    pub(crate) require_observability: bool,
    pub(crate) require_product_cohesion: bool,
    pub(crate) jobs: Option<usize>,
}

pub(crate) fn run(args: RunArgs) -> Result<i32, String> {
    let started = std::time::Instant::now();
    let root = args.root.clone();
    let receipt = args.receipt.clone();
    let target_repo = args.target_repo.clone();
    let red_report = args.red_report.clone();
    let options = crate::audit::AuditOptions {
        root: args.root,
        receipt: args.receipt,
        red_report: args.red_report,
        target_repo: args.target_repo,
        mode: args.mode,
        require_observability: args.require_observability,
        require_product_cohesion: args.require_product_cohesion,
        jobs: args.jobs,
        command_text: command_text(&root),
    };
    let result = crate::audit::run(options);
    match result {
        Ok(code) => {
            observability::write_all(
                &root,
                &receipt,
                red_report.as_deref(),
                code,
                target_repo.is_some(),
                None,
                observability::RuntimeFacts::from_elapsed_ms(elapsed_ms(started)),
            )?;
            Ok(code)
        }
        Err(err) => {
            let _ = observability::write_all(
                &root,
                &receipt,
                red_report.as_deref(),
                1,
                target_repo.is_some(),
                Some(&err),
                observability::RuntimeFacts::from_elapsed_ms(elapsed_ms(started)),
            );
            Err(err)
        }
    }
}

pub(crate) fn run_red_fixture_report(root: &Path, report: &Path) -> Result<i32, String> {
    let started = std::time::Instant::now();
    let report = red_report_claim_path(root, report)?;
    let scheduler_metrics = generate_red_fixture_report(root, &report)?;
    let runtime = observability::RuntimeFacts::from_elapsed_ms(elapsed_ms(started));
    observability::write_standalone_red_report(root, &report, runtime, &scheduler_metrics)
}

fn red_report_claim_path(root: &Path, report: &Path) -> Result<PathBuf, String> {
    if report.file_name().and_then(|name| name.to_str()) != Some("red-fixture-report.json") {
        return Err("--report basename must be red-fixture-report.json".to_string());
    }
    crate::output_path::claim_artifact_path(root, report, "red fixture report")
}

fn generate_red_fixture_report(
    root: &Path,
    report: &Path,
) -> Result<Vec<crate::scheduler::Metrics>, String> {
    let store = crate::schema_catalog::load(root);
    let validator_artifacts = crate::audit::artifacts::validator_artifacts(root)?;
    let validator_digests = crate::audit::artifacts::digest_map(&validator_artifacts);
    let scheduler = crate::scheduler::SchedulerConfig::from_jobs(None)?;
    let results = crate::red::fixtures::red_fixture_results_with_scheduler(
        root,
        &store,
        &validator_digests,
        scheduler,
    );
    let status = crate::audit::package::outputs::red_report_status(&results.rows);
    crate::audit::package::outputs::write_red_report(root, report, status, &results.rows)?;
    Ok(results.scheduler_metrics)
}

#[cfg(test)]
pub(crate) fn red_report_stdout_contract_for_test(value: &serde_json::Value) -> Vec<String> {
    observability::stdout_contract_for_test(value)
}

pub(crate) fn emit_parse_error_observability(
    root: &Path,
    raw: &[String],
    err: &str,
    elapsed_ms: u64,
) -> Result<(), String> {
    let Some(target_repo) = audit_parse_target(raw) else {
        return Ok(());
    };
    observability::emit_receipt(
        root,
        observability::ReceiptFields {
            command: if target_repo {
                "ultragoal target-repo"
            } else {
                "ultragoal source"
            },
            subcommand: "audit",
            operation: if target_repo {
                "target-repo.audit"
            } else {
                "source.audit"
            },
            surface: if target_repo { "target_repo" } else { "source" },
            check_id: "source-audit-parser-rejection-observability",
            claim_id: if target_repo {
                "target_repo_audit"
            } else {
                "source_audit"
            },
            artifact_path: "validator/src/lib.rs",
            receipt_path: if target_repo {
                "validation_artifacts/observability/target-repo-audit.json"
            } else {
                observability::SOURCE_RECEIPT
            },
            status: "fail",
            failure_class: "audit_parse_rejection",
            why_failed: &format!("audit parser rejected command: {err}"),
            where_failed: if target_repo {
                "target-repo.audit.parse"
            } else {
                "source.audit.parse"
            },
            next_repair: parse_error_next_repair(target_repo),
            claim_impact: "audit_parse_rejection_blocks_readiness_release_completion_update_goal",
            supported_claims: Vec::new(),
            runtime: parser_rejection_runtime(elapsed_ms),
        },
    )
    .map(|_| ())
}

fn audit_parse_target(raw: &[String]) -> Option<bool> {
    match raw {
        [first, ..] if first == "audit" => Some(false),
        [first, second, ..] if first == "source" && second == "audit" => Some(false),
        [first, second, ..] if first == "target-repo" && second == "audit" => Some(true),
        _ => None,
    }
}

fn parse_error_next_repair(target_repo: bool) -> &'static str {
    if target_repo {
        "repair target-repo audit arguments, rerun the narrow command, then query this run by run_id/correlation_id"
    } else {
        "repair source audit arguments, rerun the narrow command, then query this run by run_id/correlation_id"
    }
}

fn parser_rejection_runtime(duration_ms: u64) -> crate::cli::observe::telemetry::RuntimeTelemetry {
    crate::cli::observe::telemetry::RuntimeTelemetry {
        duration_ms: duration_ms.max(1),
        worker_count: 0,
        task_count: 0,
        queue_depth: 0,
        cpu_ms: None,
        memory_bytes: None,
        io_bytes: None,
        cache_mode: "parser_rejection_no_cache".to_string(),
        resource_measurement_status: "parse_rejected_before_scheduler_metrics".to_string(),
        retry_count: 0,
        backoff_ms: 0,
        saturation_status: "no_scheduler_tasks_started".to_string(),
        repair_anchor_before: "audit_parse_start".to_string(),
        repair_anchor_after: "audit_parse_rejection_telemetry_emit".to_string(),
    }
}

fn elapsed_ms(started: std::time::Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis())
        .unwrap_or(u64::MAX)
        .max(1)
}

fn command_text(root: &Path) -> String {
    let raw = std::env::args().collect::<Vec<_>>().join(" ");
    raw.replace(&root.to_string_lossy().to_string(), ".")
}
