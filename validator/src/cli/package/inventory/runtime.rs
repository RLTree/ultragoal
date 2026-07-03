use crate::cli::observe::telemetry::RuntimeTelemetry;

pub(super) fn from_metrics(
    metrics: &[crate::scheduler::Metrics],
    duration_ms: u64,
) -> RuntimeTelemetry {
    let worker_count = metrics
        .iter()
        .map(|row| row.worker_count)
        .max()
        .unwrap_or(1);
    let task_count = metrics.iter().map(|row| row.task_count).sum();
    let queue_depth = metrics.iter().map(|row| row.queue_depth).sum();
    RuntimeTelemetry {
        duration_ms,
        worker_count,
        task_count,
        queue_depth,
        cpu_ms: None,
        memory_bytes: None,
        io_bytes: None,
        cache_mode: "package_inventory_no_cache".to_string(),
        resource_measurement_status: "package_inventory_scheduler_metrics_only".to_string(),
        retry_count: 0,
        backoff_ms: 0,
        saturation_status: saturation(worker_count, task_count, queue_depth),
        repair_anchor_before: "package_inventory_check_start".to_string(),
        repair_anchor_after: "package_inventory_observability_emit".to_string(),
    }
}

fn saturation(worker_count: usize, task_count: usize, queue_depth: usize) -> String {
    if task_count == 0 {
        "no_scheduler_tasks_started".to_string()
    } else if worker_count == 0 {
        "scheduler_worker_count_missing".to_string()
    } else if queue_depth > worker_count {
        "queued_parallel_package_inventory_checks".to_string()
    } else {
        "within_worker_capacity".to_string()
    }
}
