pub(super) fn serial(
    duration_ms: u64,
    mode: &str,
    requested_jobs: usize,
) -> crate::cli::observe::telemetry::RuntimeTelemetry {
    crate::cli::observe::telemetry::RuntimeTelemetry {
        duration_ms,
        worker_count: 1,
        task_count: 1,
        queue_depth: 1,
        cpu_ms: None,
        memory_bytes: None,
        io_bytes: None,
        cache_mode: mode.to_string(),
        resource_measurement_status: format!(
            "coverage_command_wall_time_only_cpu_memory_io_unavailable_requested_jobs_{requested_jobs}"
        ),
        retry_count: 0,
        backoff_ms: 0,
        saturation_status: "shared_authority_write_serial_for_coverage_report_and_receipt_outputs"
            .to_string(),
        repair_anchor_before: "coverage_prove_start".to_string(),
        repair_anchor_after: "coverage_prove_observability_emit".to_string(),
    }
}
