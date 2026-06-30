use crate::audit::package::{checks, outputs, targets};
use crate::audit::{AuditOptions, artifacts};
use crate::scheduler::SchedulerConfig;
use crate::schema_catalog;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;

mod semantic_valid;
pub(crate) use semantic_valid::ready_artifacts;
#[cfg(test)]
pub(crate) use semantic_valid::{
    check as semantic_valid_fixture_check, checks as semantic_valid_fixture_checks,
};

pub fn run(options: AuditOptions, red_report: PathBuf) -> Result<i32, String> {
    let start = crate::audit::clock::now_iso();
    let store = schema_catalog::load(&options.root);
    let check_ids = artifacts::check_ids(&store);
    let validator_artifacts = artifacts::validator_artifacts(&options.root)?;
    let validator_digests = artifacts::digest_map(&validator_artifacts);
    let scheduler = SchedulerConfig::from_jobs(options.jobs)?;
    let check_results = checks::checks_with_scheduler(
        &options.root,
        &store,
        &check_ids,
        &validator_artifacts,
        scheduler,
    );
    let mut failures = check_results.failures;
    let mut scheduler_metrics = check_results.scheduler_metrics;
    collect_failures(
        &options,
        &store,
        &validator_digests,
        &validator_artifacts,
        &mut failures,
    );
    let (red, red_metrics) = collect_red(
        &options,
        &store,
        &validator_digests,
        &mut failures,
        scheduler,
    );
    scheduler_metrics.extend(red_metrics);
    let target_artifacts =
        targets::collect(&options, &red_report, &validator_artifacts, &mut failures);
    targets::validate(&store, &mut failures, &target_artifacts);
    checks::final_hygiene_check(&options.root, &mut failures);
    let status = outputs::package_status(&failures);
    let red_status = outputs::red_report_status(&red);
    outputs::write_red_report(&options.root, &red_report, red_status, &red)?;
    let (stdout, stderr) = outputs::write_stdio_receipts(&options.receipt, status, red.len())?;
    outputs::write_validator_receipt(outputs::ReceiptParts {
        options,
        red_report,
        stdout,
        stderr,
        check_ids,
        failures,
        red,
        target_artifacts,
        start,
        status,
        validator_artifacts,
        scheduler_metrics,
    })
}

fn collect_failures(
    options: &AuditOptions,
    store: &schema_catalog::SchemaStore,
    validator_digests: &BTreeMap<String, String>,
    validator_artifacts: &[Value],
    failures: &mut BTreeMap<String, Vec<String>>,
) {
    for error in &store.errors {
        push_failure(
            failures,
            "schema-valid",
            format!("schema_bootstrap_failed: {error}"),
        );
    }
    semantic_valid::checks(
        &options.root,
        validator_digests,
        validator_artifacts,
        failures,
    );
}

fn collect_red(
    options: &AuditOptions,
    store: &schema_catalog::SchemaStore,
    validator_digests: &BTreeMap<String, String>,
    failures: &mut BTreeMap<String, Vec<String>>,
    scheduler: SchedulerConfig,
) -> (BTreeMap<String, Value>, Vec<crate::scheduler::Metrics>) {
    let red_results = crate::red::fixtures::red_fixture_results_with_scheduler(
        &options.root,
        store,
        validator_digests,
        scheduler,
    );
    let red = red_results.rows;
    if red.is_empty() {
        push_failure(
            failures,
            "red-fixture-coverage",
            "red catalog or materialization unavailable",
        );
    }
    if red
        .values()
        .any(|row| row.get("status").and_then(Value::as_str) != Some("pass"))
    {
        push_failure(
            failures,
            "red-fixture-coverage",
            "one or more red fixtures did not fail as expected",
        );
    }
    (red, red_results.scheduler_metrics)
}

fn push_failure(
    failures: &mut BTreeMap<String, Vec<String>>,
    check_id: &str,
    detail: impl Into<String>,
) {
    failures
        .entry(check_id.to_string())
        .or_default()
        .push(detail.into());
}
