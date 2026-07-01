use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::Instant;

pub(super) const OPERATION: &str = "review-round.verify";
const CHECK_ID: &str = "review-round-verify-observability-binding";
const CLAIM_ID: &str = "review_round_source_local_observability";
const DEFAULT_RECEIPT: &str = "validation_artifacts/observability/review-round-verify.json";

pub(crate) fn observability_receipt(args: &[String]) -> Result<PathBuf, String> {
    let path = args
        .windows(2)
        .find(|window| window[0] == "--observability-receipt")
        .map(|window| PathBuf::from(&window[1]))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_RECEIPT));
    if path.as_os_str().is_empty() {
        return Err("review round observability receipt path is empty".to_string());
    }
    if path.is_absolute()
        || path
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return Err(format!(
            "{}: review round observability receipt escapes package root",
            path.display()
        ));
    }
    Ok(path)
}

pub(crate) fn run(
    root: PathBuf,
    receipt: PathBuf,
    validator_receipt: PathBuf,
    review_target_receipt: PathBuf,
    archive_receipt: PathBuf,
    observability_receipt: PathBuf,
) -> Result<i32, String> {
    let started = Instant::now();
    let anchors = crate::review::round::AnchorPaths {
        validator_receipt: validator_receipt.clone(),
        review_target_receipt: review_target_receipt.clone(),
        archive_receipt: archive_receipt.clone(),
    };
    let result = crate::review::round::validate_files(&root, &receipt, &anchors);
    let status = if result.is_ok() { "pass" } else { "fail" };
    let why_failed = result.err().unwrap_or_else(|| "none".to_string());
    let observability = observability(
        Inputs {
            root: &root,
            receipt: &receipt,
            validator_receipt: &validator_receipt,
            review_target_receipt: &review_target_receipt,
            archive_receipt: &archive_receipt,
            observability_receipt: &observability_receipt,
        },
        started,
        status,
        &why_failed,
    )?;
    crate::json_boundary::write_json(&root.join(&observability_receipt), &observability)?;
    super::review_round_stdout::print_summary(&observability);
    Ok(i32::from(status != "pass"))
}

struct Inputs<'a> {
    root: &'a Path,
    receipt: &'a Path,
    validator_receipt: &'a Path,
    review_target_receipt: &'a Path,
    archive_receipt: &'a Path,
    observability_receipt: &'a Path,
}

fn observability(
    input: Inputs<'_>,
    started: Instant,
    status: &str,
    why_failed: &str,
) -> Result<Value, String> {
    let receipt_path = input.observability_receipt.to_string_lossy().to_string();
    let artifact_path = format!(
        "{},{},{},{}",
        input.receipt.display(),
        input.validator_receipt.display(),
        input.review_target_receipt.display(),
        input.archive_receipt.display()
    );
    crate::cli::observe::telemetry::command_receipt(
        input.root,
        crate::cli::observe::telemetry::CommandTelemetry {
            command: "ultragoal review-round",
            subcommand: "verify",
            operation: OPERATION,
            surface: "review_round",
            law_id: "typed-review-round-receipt-authority",
            check_id: CHECK_ID,
            claim_id: CLAIM_ID,
            artifact_path: &artifact_path,
            receipt_path: &receipt_path,
            status,
            failure_class: failure_class(status),
            why_failed,
            where_failed: where_failed(status),
            next_repair: next_repair(status),
            claim_impact: claim_impact(status),
            blocked_claims: blocked_claims(),
            supported_claims: supported_claims(status),
            runtime: Some(runtime(started)),
            emit: true,
        },
    )
}

fn runtime(started: Instant) -> crate::cli::observe::telemetry::RuntimeTelemetry {
    crate::cli::observe::telemetry::RuntimeTelemetry {
        duration_ms: u64::try_from(started.elapsed().as_millis())
            .unwrap_or(u64::MAX)
            .max(1),
        worker_count: 1,
        task_count: 4,
        queue_depth: 4,
        cpu_ms: None,
        memory_bytes: None,
        io_bytes: None,
        cache_mode: "review_round_anchor_receipts_no_cache".to_string(),
        resource_measurement_status: "wall_time_only_cpu_memory_io_unavailable".to_string(),
        retry_count: 0,
        backoff_ms: 0,
        saturation_status: "shared_authority_read_serial_review_round_anchors".to_string(),
        repair_anchor_before: "review_round_validate_files_start".to_string(),
        repair_anchor_after: "review_round_observability_emit".to_string(),
    }
}

fn failure_class(status: &str) -> &'static str {
    match status {
        "pass" => "none",
        _ => "review_round_validation_failure",
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
        "query this run through observe logs/metrics/traces, repair the named review-round receipt or stale anchor binding, then rerun review-round verify"
    }
}

fn claim_impact(status: &str) -> &'static str {
    match status {
        "pass" => "supports_review_round_source_local_observability_only",
        _ => "review_round_failed_blocks_readiness_release_completion_update_goal",
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
