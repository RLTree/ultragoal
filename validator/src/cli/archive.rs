use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::Instant;

pub(super) const OPERATION: &str = "archive.build";
const CHECK_ID: &str = "archive-build-observability-binding";
pub(super) const CLAIM_ID: &str = "archive_source_local_observability";
#[cfg(test)]
const DEFAULT_RECEIPT: &str = "validation_artifacts/observability/archive-build.json";
const BLOCKED_CLAIMS: &str = "completion,readiness,release,reviewer_exposure,app_registry_exposure,final_packet_correctness,update_goal_eligibility";

#[cfg(test)]
pub(crate) fn parse(raw: &[String]) -> Result<crate::Command, String> {
    let args = strip_build_or_verify(raw);
    Ok(crate::Command::Archive {
        zip: required_path(args, "--zip")?,
        receipt: required_path(args, "--receipt")?,
        observability_receipt: observability_receipt(args)?,
        zip_root: opt_string(args, "--zip-root")
            .unwrap_or_else(|| "harness-ultragoal-plugin-proposal".to_string()),
        archive_purpose: opt_string(args, "--archive-purpose")
            .or_else(|| opt_string(args, "--purpose"))
            .unwrap_or_else(|| "candidate_review_anchor".to_string()),
    })
}

#[cfg(test)]
pub(crate) fn observability_receipt(args: &[String]) -> Result<PathBuf, String> {
    let path = args
        .windows(2)
        .find(|window| window[0] == "--observability-receipt")
        .map(|window| PathBuf::from(&window[1]))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_RECEIPT));
    if path.as_os_str().is_empty() {
        return Err("archive observability receipt path is empty".to_string());
    }
    if path.is_absolute()
        || path
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return Err(format!(
            "{}: archive observability receipt escapes package root",
            path.display()
        ));
    }
    Ok(path)
}

#[cfg(test)]
fn strip_build_or_verify(args: &[String]) -> &[String] {
    match args.first().map(String::as_str) {
        Some("build" | "verify") => &args[1..],
        _ => args,
    }
}

#[cfg(test)]
fn required_path(args: &[String], key: &str) -> Result<PathBuf, String> {
    opt_string(args, key)
        .map(PathBuf::from)
        .ok_or_else(|| format!("missing required argument {key}"))
}

#[cfg(test)]
fn opt_string(args: &[String], key: &str) -> Option<String> {
    args.windows(2)
        .find(|window| window[0] == key)
        .map(|window| window[1].clone())
}

pub(crate) fn run(
    root: PathBuf,
    zip: PathBuf,
    receipt: PathBuf,
    observability_receipt: PathBuf,
    zip_root: String,
    archive_purpose: String,
) -> Result<i32, String> {
    let started = Instant::now();
    let zip = crate::output_path::claim_artifact_path(&root, &zip, "archive zip")?;
    let receipt = crate::output_path::claim_artifact_path(&root, &receipt, "archive receipt")?;
    let outcome = build_and_write(&root, &zip, &receipt, &zip_root, &archive_purpose);
    let status = outcome.status();
    let observability = observability(
        &root,
        &zip,
        &receipt,
        &observability_receipt,
        started,
        &outcome,
    )?;
    let observability_path = crate::output_path::claim_artifact_path(
        &root,
        &observability_receipt,
        "archive observability receipt",
    )?;
    crate::json_boundary::write_json(&observability_path, &observability)
        .map_err(|err| crate::cli::observe::telemetry::redact_sensitive_text(&err))?;
    super::archive_stdout::print_summary(&observability, outcome.archive_digest());
    Ok(i32::from(status != "pass"))
}

enum Outcome {
    Pass(Value),
    Fail(String),
}

impl Outcome {
    fn status(&self) -> &'static str {
        match self {
            Self::Pass(_) => "pass",
            Self::Fail(_) => "fail",
        }
    }

    fn why_failed(&self) -> &str {
        match self {
            Self::Pass(_) => "none",
            Self::Fail(error) => error,
        }
    }

    fn archive_digest(&self) -> &str {
        match self {
            Self::Pass(value) => value["archive"]["digest"].as_str().unwrap_or("<missing>"),
            Self::Fail(_) => "<missing>",
        }
    }
}

fn build_and_write(
    root: &Path,
    zip: &Path,
    claim_receipt_path: &Path,
    zip_root: &str,
    archive_purpose: &str,
) -> Outcome {
    let value = match crate::archive::build_archive(root, zip, zip_root, archive_purpose) {
        Ok(value) => value,
        Err(err) => return Outcome::Fail(err),
    };
    match crate::json_boundary::write_json(claim_receipt_path, &value) {
        Ok(()) => Outcome::Pass(value),
        Err(err) => Outcome::Fail(err),
    }
}

fn observability(
    root: &Path,
    zip: &Path,
    receipt: &Path,
    observability_receipt: &Path,
    started: Instant,
    outcome: &Outcome,
) -> Result<Value, String> {
    let candidate = crate::package::inventory::package_digest(root)
        .unwrap_or_else(|_| crate::digest::ZERO.to_string());
    let receipt_path = observability_receipt.to_string_lossy().to_string();
    let artifact_path = format!(
        "plugin-manifest-draft.json,{},{}",
        zip.display(),
        receipt.display()
    );
    crate::cli::observe::telemetry::command_receipt_for_candidate(
        root,
        crate::cli::observe::telemetry::CommandTelemetry {
            command: "ultragoal archive",
            subcommand: "build",
            operation: OPERATION,
            surface: "archive",
            law_id: "full-local-observability-stack-integration-non-opaque-failure",
            check_id: CHECK_ID,
            claim_id: CLAIM_ID,
            artifact_path: &artifact_path,
            receipt_path: &receipt_path,
            status: outcome.status(),
            failure_class: failure_class(outcome.status()),
            why_failed: outcome.why_failed(),
            where_failed: where_failed(outcome.status()),
            next_repair: next_repair(outcome.status()),
            claim_impact: claim_impact(outcome.status()),
            blocked_claims: blocked_claims(),
            supported_claims: supported_claims(outcome.status()),
            runtime: Some(runtime(started)),
            emit: true,
        },
        candidate,
    )
}

fn runtime(started: Instant) -> crate::cli::observe::telemetry::RuntimeTelemetry {
    crate::cli::observe::telemetry::RuntimeTelemetry {
        duration_ms: u64::try_from(started.elapsed().as_millis())
            .unwrap_or(u64::MAX)
            .max(1),
        worker_count: 1,
        task_count: 3,
        queue_depth: 3,
        cpu_ms: None,
        memory_bytes: None,
        io_bytes: None,
        cache_mode: "archive_digest_no_cache".to_string(),
        resource_measurement_status: "wall_time_only_cpu_memory_io_unavailable".to_string(),
        retry_count: 0,
        backoff_ms: 0,
        saturation_status: "shared_authority_write_serial_archive_zip_and_receipt".to_string(),
        repair_anchor_before: "archive_build_start".to_string(),
        repair_anchor_after: "archive_observability_emit".to_string(),
    }
}

fn failure_class(status: &str) -> &'static str {
    match status {
        "pass" => "none",
        _ => "archive_build_failure",
    }
}

fn where_failed(status: &str) -> &'static str {
    if status == "pass" { "none" } else { OPERATION }
}

fn next_repair(status: &str) -> &'static str {
    if status == "pass" {
        "none"
    } else {
        "query this run through observe logs/metrics/traces, repair archive manifest closure, archive input hygiene, or receipt output, then rerun archive build"
    }
}

fn claim_impact(status: &str) -> &'static str {
    match status {
        "pass" => "supports_archive_source_local_observability_only",
        _ => "archive_build_failed_blocks_readiness_release_completion_update_goal",
    }
}

fn supported_claims(status: &str) -> Vec<String> {
    match status {
        "pass" => vec![CLAIM_ID.to_string()],
        _ => Vec::new(),
    }
}

fn blocked_claims() -> Vec<String> {
    BLOCKED_CLAIMS.split(',').map(ToString::to_string).collect()
}
