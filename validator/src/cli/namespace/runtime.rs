use std::collections::BTreeSet;

pub(super) fn from_metrics(
    rows: &[crate::scheduler::Metrics],
    duration_ms: u64,
) -> crate::cli::observe::telemetry::RuntimeTelemetry {
    crate::cli::observe::telemetry::RuntimeTelemetry {
        duration_ms,
        worker_count: rows.iter().map(|row| row.worker_count).max().unwrap_or(0),
        task_count: rows.iter().map(|row| row.task_count).sum(),
        queue_depth: rows.iter().map(|row| row.queue_depth).sum(),
        cpu_ms: sum_u128(rows, |row| row.cpu_ms),
        memory_bytes: sum_u64(rows, |row| row.memory_bytes),
        io_bytes: sum_u64(rows, |row| row.io_bytes),
        cache_mode: unique_text(rows, |row| row.cache_mode, "namespace_no_cache"),
        resource_measurement_status: unique_text(
            rows,
            |row| row.resource_measurement_status,
            "namespace_no_scheduler_metrics",
        ),
        retry_count: 0,
        backoff_ms: 0,
        saturation_status: saturation_status(rows),
        repair_anchor_before: "namespace_check_start".to_string(),
        repair_anchor_after: "namespace_observability_emit".to_string(),
    }
}

fn sum_u128(
    rows: &[crate::scheduler::Metrics],
    field: fn(&crate::scheduler::Metrics) -> Option<u128>,
) -> Option<u64> {
    let mut total = 0_u128;
    let mut any = false;
    for row in rows {
        if let Some(value) = field(row) {
            total = total.saturating_add(value);
            any = true;
        }
    }
    any.then(|| u64::try_from(total).unwrap_or(u64::MAX))
}

fn sum_u64(
    rows: &[crate::scheduler::Metrics],
    field: fn(&crate::scheduler::Metrics) -> Option<u64>,
) -> Option<u64> {
    let mut total = 0_u64;
    let mut any = false;
    for row in rows {
        if let Some(value) = field(row) {
            total = total.saturating_add(value);
            any = true;
        }
    }
    any.then_some(total)
}

fn unique_text(
    rows: &[crate::scheduler::Metrics],
    field: fn(&crate::scheduler::Metrics) -> &'static str,
    fallback: &str,
) -> String {
    let values = rows.iter().map(field).collect::<BTreeSet<_>>();
    match values.len() {
        0 => fallback.to_string(),
        1 => values.iter().next().expect("one value").to_string(),
        _ => "mixed_namespace_scheduler_modes".to_string(),
    }
}

fn saturation_status(rows: &[crate::scheduler::Metrics]) -> String {
    let task_count = rows.iter().map(|row| row.task_count).sum::<usize>();
    let queue_depth = rows.iter().map(|row| row.queue_depth).sum::<usize>();
    let worker_count = rows.iter().map(|row| row.worker_count).max().unwrap_or(0);
    if task_count == 0 {
        "no_scheduler_tasks_started".to_string()
    } else if worker_count == 0 {
        "scheduler_worker_count_missing".to_string()
    } else if queue_depth > worker_count {
        "queued_parallel_namespace_checks".to_string()
    } else {
        "within_worker_capacity".to_string()
    }
}

#[cfg(test)]
mod tests {
    fn metric(
        worker_count: usize,
        task_count: usize,
        queue_depth: usize,
        cache_mode: &'static str,
    ) -> crate::scheduler::Metrics {
        crate::scheduler::Metrics {
            task_class: crate::scheduler::TaskClass::PureReadParallel.id(),
            worker_count,
            task_count,
            queue_depth,
            wall_ms: 1,
            cpu_ms: Some(u128::from(task_count as u64)),
            memory_bytes: Some(task_count as u64),
            io_bytes: Some(queue_depth as u64),
            cache_mode,
            resource_measurement_status: "measured",
            deterministic_ordering: true,
            shared_validation_artifact_writes_allowed: false,
        }
    }

    #[test]
    fn namespace_runtime_aggregates_optional_resource_metrics() {
        let rows = [metric(2, 2, 4, "warm"), metric(1, 1, 1, "cold")];
        let runtime = super::from_metrics(&rows, 9);
        assert_eq!(runtime.duration_ms, 9);
        assert_eq!(runtime.worker_count, 2);
        assert_eq!(runtime.task_count, 3);
        assert_eq!(runtime.queue_depth, 5);
        assert_eq!(runtime.cpu_ms, Some(3));
        assert_eq!(runtime.memory_bytes, Some(3));
        assert_eq!(runtime.io_bytes, Some(5));
        assert_eq!(runtime.cache_mode, "mixed_namespace_scheduler_modes");
        assert_eq!(
            runtime.saturation_status,
            "queued_parallel_namespace_checks"
        );
    }

    #[test]
    fn namespace_runtime_reports_empty_and_missing_worker_states() {
        let empty = super::from_metrics(&[], 1);
        assert_eq!(empty.worker_count, 0);
        assert_eq!(empty.saturation_status, "no_scheduler_tasks_started");
        assert_eq!(empty.cache_mode, "namespace_no_cache");

        let mut missing_worker = metric(0, 1, 1, "warm");
        missing_worker.cpu_ms = None;
        missing_worker.memory_bytes = None;
        missing_worker.io_bytes = None;
        let runtime = super::from_metrics(&[missing_worker], 1);
        assert_eq!(runtime.cpu_ms, None);
        assert_eq!(runtime.memory_bytes, None);
        assert_eq!(runtime.io_bytes, None);
        assert_eq!(runtime.saturation_status, "scheduler_worker_count_missing");
    }
}
