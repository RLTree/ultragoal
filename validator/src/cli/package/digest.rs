use std::path::Path;
use std::time::Instant;

const RECEIPT_REL: &str = "validation_artifacts/observability/package-digest.json";

pub(crate) fn run(root: &Path) -> Result<i32, String> {
    let started = Instant::now();
    let digest_result = crate::package::inventory::package_digest(root);
    let status = if digest_result.is_ok() {
        "pass"
    } else {
        "fail"
    };
    let why_failed = digest_result
        .as_ref()
        .err()
        .map(|err| format!("package digest failed: {err}"))
        .unwrap_or_else(|| "none".to_string());
    let (failure_class, where_failed, next_repair, claim_impact, supported_claims) =
        if digest_result.is_ok() {
            (
                "none",
                "none",
                "none",
                "supports_source_package_digest_only",
                vec!["source_package_digest".to_string()],
            )
        } else {
            (
                "package_digest_failure",
                "package.digest",
                "repair package manifest/resource paths, rerun package digest, then query this run by run_id/correlation_id",
                "package_digest_failed_blocks_readiness_release_completion_update_goal",
                Vec::new(),
            )
        };
    let candidate = digest_result.unwrap_or_else(|_| crate::digest::ZERO.to_string());
    let value = crate::cli::observe::telemetry::command_receipt_for_candidate(
        root,
        crate::cli::observe::telemetry::CommandTelemetry {
            command: "ultragoal package",
            subcommand: "digest",
            operation: "package.digest",
            surface: "source_package",
            law_id: crate::cli::observe::types::LAW_ID,
            check_id: "package-digest-observability-binding",
            claim_id: "source_package_digest",
            artifact_path: "plugin-manifest-draft.json",
            receipt_path: RECEIPT_REL,
            status,
            failure_class,
            why_failed: &why_failed,
            where_failed,
            next_repair,
            claim_impact,
            blocked_claims: blocked_claims(),
            supported_claims,
            runtime: Some(runtime(started)),
            emit: true,
        },
        candidate,
    )?;
    let receipt_path = crate::output_path::literal_claim_artifact_path(
        root,
        RECEIPT_REL,
        "package digest receipt",
    );
    crate::json_boundary::write_json(&receipt_path, &value)?;
    print_receipt(&value);
    Ok(i32::from(status != "pass"))
}

fn print_receipt(value: &serde_json::Value) {
    for line in stdout_contract(value) {
        println!("{line}");
    }
}

#[cfg(test)]
pub(crate) fn stdout_contract_for_test(value: &serde_json::Value) -> Vec<String> {
    stdout_contract(value)
}

fn stdout_contract(value: &serde_json::Value) -> Vec<String> {
    let status = value["status"].as_str().unwrap_or("fail");
    let digest = value["candidate_digest"].as_str().unwrap_or("<missing>");
    let mut lines = Vec::new();
    if status == "pass" {
        lines.push(digest.to_string());
    }
    lines.push(format!(
        "ultragoal-package-digest {status} proven={} candidate={digest} receipt={RECEIPT_REL} run_id={} correlation_id={} claim_impact={} supported_claims={} unsupported_claims={}",
        if status == "pass" { "source_package_digest" } else { "none" },
        value["run_id"].as_str().unwrap_or("<missing>"),
        value["correlation_id"].as_str().unwrap_or("<missing>"),
        value["claim_impact"].as_str().unwrap_or("<missing>"),
        csv(value.get("supported_claims")),
        csv(value.get("blocked_claims")),
    ));
    if status != "pass" {
        lines.push(format!(
            "failed_check={} why={} where={} claim_impact={} next_repair={}",
            value["check_id"].as_str().unwrap_or("<missing>"),
            value["why_failed"].as_str().unwrap_or("<missing>"),
            value["where_failed"].as_str().unwrap_or("<missing>"),
            value["claim_impact"].as_str().unwrap_or("<missing>"),
            value["next_repair"].as_str().unwrap_or("<missing>")
        ));
    }
    lines
}

fn runtime(started: Instant) -> crate::cli::observe::telemetry::RuntimeTelemetry {
    crate::cli::observe::telemetry::RuntimeTelemetry {
        duration_ms: u64::try_from(started.elapsed().as_millis())
            .unwrap_or(u64::MAX)
            .max(1),
        worker_count: 1,
        task_count: 1,
        queue_depth: 0,
        cpu_ms: None,
        memory_bytes: None,
        io_bytes: None,
        cache_mode: "package_digest_no_cache".to_string(),
        resource_measurement_status: "wall_time_only_cpu_memory_io_unavailable".to_string(),
        retry_count: 0,
        backoff_ms: 0,
        saturation_status: "serial_command_typed".to_string(),
        repair_anchor_before: "package_digest_command_start".to_string(),
        repair_anchor_after: "package_digest_observability_emit".to_string(),
    }
}

fn csv(value: Option<&serde_json::Value>) -> String {
    value
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<Vec<_>>()
                .join(",")
        })
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| "none".to_string())
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
