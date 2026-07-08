pub(crate) mod agent;
pub(crate) mod artifacts;
pub(crate) mod cli;
pub(crate) mod clock;
pub(crate) mod contract;
pub(crate) mod coverage;
pub(crate) mod final_packet;
pub(crate) mod fit_repo_receipt;
pub(crate) mod foundational_law_trace;
pub(crate) mod halo;
pub(crate) mod improvement_loop;
pub(crate) mod law;
pub(crate) mod mandatory;
pub(crate) mod namespace;
pub(crate) mod observability;
pub(crate) mod openai;
pub(crate) mod package;
pub(crate) mod plugin;
pub(crate) mod product;
pub(crate) mod promptfoo;
pub(crate) mod receipt;
pub(crate) mod red;
pub(crate) mod research;
pub(crate) mod review_history;
pub(crate) mod rust;
pub(crate) mod session;
pub(crate) use session::log::hardening as session_log_hardening;
pub(crate) mod source_obligations;
pub(crate) mod standards_gardening;
pub(crate) mod template_integrity;
pub(crate) mod text_guards;

use crate::json_boundary;
use crate::target_repo;
use std::path::PathBuf;

pub struct AuditOptions {
    pub root: PathBuf,
    pub receipt: PathBuf,
    pub red_report: Option<PathBuf>,
    pub target_repo: Option<PathBuf>,
    pub mode: String,
    pub require_observability: bool,
    pub require_product_cohesion: bool,
    pub jobs: Option<usize>,
    pub command_text: String,
}

pub fn run(options: AuditOptions) -> Result<i32, String> {
    if !receipt::speed::is_known_mode(&options.mode) {
        return Err(format!("invalid source audit --mode: {}", options.mode));
    }
    if let Some(target_repo) = &options.target_repo {
        return run_target_repo(&options, target_repo);
    }
    let red_report = red_report_path(&options);
    if red_report.file_name().and_then(|name| name.to_str()) != Some("red-fixture-report.json") {
        return Err("--red-report basename must be red-fixture-report.json".to_string());
    }
    package::run::run(options, red_report)
}

pub(crate) fn red_report_path(options: &AuditOptions) -> PathBuf {
    options
        .red_report
        .clone()
        .unwrap_or_else(|| options.receipt.with_file_name("red-fixture-report.json"))
}

fn run_target_repo(
    options: &AuditOptions,
    target_repo_path: &std::path::Path,
) -> Result<i32, String> {
    let validator_artifacts = artifacts::validator_artifacts(&options.root)?;
    let (target_receipt, code) = target_repo::audit_target_repo(
        target_repo_path,
        &options.mode,
        "ultragoal --root . target-repo audit --surface-root <target-repo> --receipt <receipt>",
        &validator_artifacts,
        options.require_observability,
        options.require_product_cohesion,
        Some(&options.root),
    );
    target_receipt_write_result(json_boundary::write_json(&options.receipt, &target_receipt))?;
    validate_target_receipt(&target_receipt)?;
    println!(
        "ultragoal-target-repo {} receipt={}",
        target_receipt["status"],
        options.receipt.display()
    );
    Ok(code)
}

pub(crate) fn validate_target_receipt(target_receipt: &serde_json::Value) -> Result<(), String> {
    let errors = target_repo::target_receipt_errors(target_receipt);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

pub(crate) fn target_receipt_write_result(result: Result<(), String>) -> Result<(), String> {
    result
}
