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

fn elapsed_ms(started: std::time::Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis())
        .unwrap_or(u64::MAX)
        .max(1)
}

fn command_text(root: &Path) -> String {
    let raw = std::env::args().collect::<Vec<_>>().join(" ");
    raw.replace(&root.to_string_lossy().to_string(), ".")
}
