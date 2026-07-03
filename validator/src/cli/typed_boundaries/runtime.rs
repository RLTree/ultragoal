pub(super) fn from_metrics(
    metrics: &[crate::scheduler::Metrics],
    elapsed_ms: u64,
) -> crate::cli::observe::telemetry::RuntimeTelemetry {
    let task_count = metrics.iter().map(|metric| metric.task_count).sum();
    let queue_depth = metrics.iter().map(|metric| metric.queue_depth).sum();
    let worker_count = metrics
        .iter()
        .map(|metric| metric.worker_count)
        .max()
        .unwrap_or(0);
    crate::cli::observe::telemetry::RuntimeTelemetry {
        duration_ms: elapsed_ms,
        worker_count,
        task_count,
        queue_depth,
        cpu_ms: None,
        memory_bytes: None,
        io_bytes: None,
        cache_mode: "typed_boundary_no_cache".to_string(),
        resource_measurement_status: "typed_boundary_scheduler_metrics_only".to_string(),
        retry_count: 0,
        backoff_ms: 0,
        saturation_status: saturation_status(worker_count, task_count, queue_depth),
        repair_anchor_before: "typed_boundary_check_start".to_string(),
        repair_anchor_after: "typed_boundary_observability_emit".to_string(),
    }
}

fn saturation_status(worker_count: usize, task_count: usize, queue_depth: usize) -> String {
    if task_count == 0 {
        "no_scheduler_tasks_started".to_string()
    } else if worker_count == 0 {
        "scheduler_worker_count_missing".to_string()
    } else if queue_depth > worker_count {
        "queued_parallel_typed_boundary_checks".to_string()
    } else {
        "within_worker_capacity".to_string()
    }
}
