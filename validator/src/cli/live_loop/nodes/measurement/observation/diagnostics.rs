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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::live_loop::surfaces::surface_by_id;

    fn command_run(exit_code: i32, status_success: bool, launch_error: bool) -> FullCommandRun {
        FullCommandRun {
            exit_code,
            status_success,
            launch_error,
            duration_ms: 17,
            stdout_digest: "sha256:stdout".to_string(),
            stderr_digest: "sha256:stderr".to_string(),
            executed_test_count: None,
            failure: Default::default(),
        }
    }

    #[test]
    fn command_observation_diagnostics_describe_failed_verified_work() {
        let surface = surface_by_id("changed_files").expect("surface");
        let launched = command_run(7, false, false);
        assert_eq!(
            command_failure_class(&launched),
            "verified_local_command_failed"
        );
        assert_eq!(
            where_failed(surface, "verified_local_command_failed"),
            "loop.measure.changed_files.verified_local_command"
        );
        assert!(
            why_failed(surface, &launched, "verified_local_command_failed")
                .contains("verified-local command exited 7")
        );
        assert!(
            next_repair(surface, "verified_local_command_failed")
                .contains("loop measure --node changed_files")
        );

        let launch_error = command_run(1, false, true);
        assert_eq!(
            command_failure_class(&launch_error),
            "verified_local_command_launch_failed"
        );
        assert!(
            why_failed(
                surface,
                &launch_error,
                "verified_local_command_launch_failed"
            )
            .contains("could not launch")
        );
        assert!(why_failed(surface, &launch_error, "unknown").contains("strict classification"));
        assert!(blocked_claims().contains(&"update_goal_eligibility".to_string()));
    }
}
