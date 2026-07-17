use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::Instant;

pub(super) const OPERATION: &str = "review-target.build";
const CHECK_ID: &str = "review-target-build-observability-binding";
pub(super) const CLAIM_ID: &str = "review_target_source_local_observability";
const DEFAULT_RECEIPT: &str = "validation_artifacts/observability/review-target-build.json";

#[cfg(test)]
pub(crate) fn observability_receipt(args: &[String]) -> Result<PathBuf, String> {
    let path = args
        .windows(2)
        .find(|window| window[0] == "--observability-receipt")
        .map(|window| PathBuf::from(&window[1]))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_RECEIPT));
    if path.as_os_str().is_empty() {
        return Err("review target observability receipt path is empty".to_string());
    }
    if path.is_absolute()
        || path
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return Err(format!(
            "{}: review target observability receipt escapes package root",
            path.display()
        ));
    }
    Ok(path)
}

pub(crate) fn run(
    root: PathBuf,
    receipt: PathBuf,
    observability_receipt: PathBuf,
) -> Result<i32, String> {
    let started = Instant::now();
    let receipt_path =
        crate::output_path::claim_artifact_path(&root, &receipt, "review target receipt")?;
    let outcome = build_and_write(&root, &receipt_path);
    let status = outcome.status();
    let observability = observability(&root, &receipt, &observability_receipt, started, &outcome)?;
    let observability_path = crate::output_path::claim_artifact_path(
        &root,
        &observability_receipt,
        "review target observability receipt",
    )?;
    crate::json_boundary::write_json(&observability_path, &observability)?;
    super::target_stdout::print_summary(&observability, outcome.review_target_digest());
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

    fn review_target_digest(&self) -> &str {
        match self {
            Self::Pass(value) => value["review_target_digest"]
                .as_str()
                .unwrap_or("<missing>"),
            Self::Fail(_) => "<missing>",
        }
    }
}

fn build_and_write(root: &Path, claim_receipt_path: &Path) -> Outcome {
    let value = match crate::package::build_review_target_receipt(root) {
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
    receipt: &Path,
    observability_receipt: &Path,
    started: Instant,
    outcome: &Outcome,
) -> Result<Value, String> {
    let candidate = crate::package::inventory::package_digest(root)
        .unwrap_or_else(|_| crate::digest::ZERO.to_string());
    let receipt_path = observability_receipt.to_string_lossy().to_string();
    let artifact_path = format!("plugin-manifest-draft.json,{}", receipt.display());
    crate::cli::observe::telemetry::command_receipt_for_candidate(
        root,
        crate::cli::observe::telemetry::CommandTelemetry {
            command: "ultragoal review-target",
            subcommand: "build",
            operation: OPERATION,
            surface: "review_target",
            law_id: "detached-review-target-identity-anchor",
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
        task_count: 2,
        queue_depth: 2,
        cpu_ms: None,
        memory_bytes: None,
        io_bytes: None,
        cache_mode: "review_target_digest_no_cache".to_string(),
        resource_measurement_status: "wall_time_only_cpu_memory_io_unavailable".to_string(),
        retry_count: 0,
        backoff_ms: 0,
        saturation_status: "shared_authority_write_serial_review_target_receipt".to_string(),
        repair_anchor_before: "review_target_build_start".to_string(),
        repair_anchor_after: "review_target_observability_emit".to_string(),
    }
}

fn failure_class(status: &str) -> &'static str {
    match status {
        "pass" => "none",
        _ => "review_target_build_failure",
    }
}

fn where_failed(status: &str) -> &'static str {
    match status {
        "pass" => "none",
        _ => OPERATION,
    }
}

fn next_repair(status: &str) -> &'static str {
    if status == "pass" {
        "none"
    } else {
        "query this run through observe logs/metrics/traces, repair review-target manifest closure or receipt output, then rerun review-target build"
    }
}

fn claim_impact(status: &str) -> &'static str {
    match status {
        "pass" => "supports_review_target_source_local_observability_only",
        _ => "review_target_failed_blocks_readiness_release_completion_update_goal",
    }
}

fn supported_claims(status: &str) -> Vec<String> {
    if status == "pass" {
        vec![CLAIM_ID.to_string()]
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
