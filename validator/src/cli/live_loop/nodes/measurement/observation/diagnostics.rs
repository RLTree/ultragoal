use crate::cli::live_loop::surfaces::LoopValidationSurface;
use crate::cli::live_loop::{LiveLoopCommand, nodes::measurement::full_command::FullCommandRun};

pub(super) fn runtime(
    surface: LoopValidationSurface,
    command: &LiveLoopCommand,
    actual_work: &FullCommandRun,
) -> crate::cli::observe::telemetry::RuntimeTelemetry {
    crate::cli::observe::telemetry::RuntimeTelemetry {
        duration_ms: actual_work.duration_ms,
        worker_count: 1,
        task_count: 1,
        queue_depth: 1,
        cpu_ms: None,
        memory_bytes: None,
        io_bytes: None,
        cache_mode: command.cache_mode.clone(),
        resource_measurement_status: "wall_time_only_cpu_memory_io_unavailable".to_string(),
        retry_count: 0,
        backoff_ms: 0,
        saturation_status: format!(
            "{};serial_reason={}",
            surface.execution_task_class.id(),
            surface.execution_serial_reason
        ),
        repair_anchor_before: "loop_measure_node_command_start".to_string(),
        repair_anchor_after: "loop_measure_node_command_exit".to_string(),
    }
}

pub(super) fn command_failure_class(actual_work: &FullCommandRun) -> &'static str {
    if actual_work.launch_error {
        "verified_local_command_launch_failed"
    } else if !actual_work.status_success {
        "verified_local_command_failed"
    } else {
        "none"
    }
}

pub(super) fn where_failed(surface: LoopValidationSurface, failure_class: &str) -> String {
    if failure_class == "none" {
        "none".to_string()
    } else {
        format!("loop.measure.{}.verified_local_command", surface.id)
    }
}

pub(super) fn why_failed(
    _surface: LoopValidationSurface,
    actual_work: &FullCommandRun,
    failure_class: &str,
) -> String {
    match failure_class {
        "none" => "none".to_string(),
        "verified_local_command_launch_failed" => {
            "verified-local command could not launch; stderr digest records the launch error"
                .to_string()
        }
        "verified_local_command_failed" => format!(
            "verified-local command exited {}; stdout_digest={} stderr_digest={}",
            actual_work.exit_code, actual_work.stdout_digest, actual_work.stderr_digest
        ),
        _ => "verified-local command observation failed strict classification".to_string(),
    }
}

pub(super) fn next_repair(surface: LoopValidationSurface, failure_class: &str) -> String {
    if failure_class == "none" {
        "none".to_string()
    } else {
        format!(
            "run `{}` directly, repair the command behavior, then rerun `target/debug/ultragoal --root . loop measure --node {} --tier hot --cache-mode verified-local`",
            surface.narrow_rerun, surface.id
        )
    }
}

pub(super) fn blocked_claims() -> Vec<String> {
    [
        "observability_live_loop_speed_claim",
        "observability_product_closure",
        "readiness",
        "release",
        "completion",
        "final_packet_correctness",
        "update_goal_eligibility",
    ]
    .into_iter()
    .map(ToString::to_string)
    .collect()
}
