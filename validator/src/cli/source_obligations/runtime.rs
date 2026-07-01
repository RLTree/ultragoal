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
        cache_mode: unique_text(rows, |row| row.cache_mode, "source_obligations_no_cache"),
        resource_measurement_status: unique_text(
            rows,
            |row| row.resource_measurement_status,
            "source_obligations_no_scheduler_metrics",
        ),
        retry_count: 0,
        backoff_ms: 0,
        saturation_status: saturation_status(rows),
        repair_anchor_before: "source_obligations_check_start".to_string(),
        repair_anchor_after: "source_obligations_observability_emit".to_string(),
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
        _ => "mixed_source_obligations_scheduler_modes".to_string(),
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
        "queued_parallel_source_obligation_checks".to_string()
    } else {
        "within_worker_capacity".to_string()
    }
}
