use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RuntimeFacts {
    elapsed_ms: u64,
}

impl RuntimeFacts {
    pub(crate) fn from_elapsed_ms(elapsed_ms: u64) -> Self {
        Self { elapsed_ms }
    }

    pub(super) fn elapsed_ms(self) -> u64 {
        self.elapsed_ms
    }
}

pub(super) fn telemetry(
    audit: &Value,
    facts: RuntimeFacts,
) -> crate::cli::observe::telemetry::RuntimeTelemetry {
    let rows = audit
        .get("scheduler_execution")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let worker_count = rows
        .iter()
        .filter_map(|row| usize_field(row, "worker_count"))
        .max();
    let task_count = rows
        .iter()
        .filter_map(|row| usize_field(row, "task_count"))
        .sum::<usize>();
    let queue_depth = rows
        .iter()
        .filter_map(|row| usize_field(row, "queue_depth"))
        .sum::<usize>();
    let cache_mode = unique_text(&rows, "cache_mode", "pre_scheduler_guard_no_cache");
    let resource_measurement_status = unique_text(
        &rows,
        "resource_measurement_status",
        "pre_scheduler_guard_no_scheduler_metrics",
    );
    crate::cli::observe::telemetry::RuntimeTelemetry {
        duration_ms: facts.elapsed_ms,
        worker_count: worker_count.unwrap_or(0),
        task_count,
        queue_depth,
        cpu_ms: sum_u64(&rows, "cpu_ms"),
        memory_bytes: sum_u64(&rows, "memory_bytes"),
        io_bytes: sum_u64(&rows, "io_bytes"),
        cache_mode,
        resource_measurement_status,
        retry_count: 0,
        backoff_ms: 0,
        saturation_status: saturation_status(worker_count.unwrap_or(0), task_count, queue_depth),
        repair_anchor_before: "source_audit_command_start".to_string(),
        repair_anchor_after: "source_audit_observability_emit".to_string(),
    }
}

fn usize_field(row: &Value, key: &str) -> Option<usize> {
    row.get(key)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

fn sum_u64(rows: &[Value], key: &str) -> Option<u64> {
    let mut sum = 0_u64;
    let mut any = false;
    for row in rows {
        let Some(value) = row.get(key) else {
            continue;
        };
        if value.is_null() {
            continue;
        }
        let value = value.as_u64()?;
        sum = sum.saturating_add(value);
        any = true;
    }
    any.then_some(sum)
}

fn unique_text(rows: &[Value], key: &str, fallback: &str) -> String {
    let values = rows
        .iter()
        .filter_map(|row| row.get(key).and_then(Value::as_str))
        .filter(|value| !value.trim().is_empty())
        .collect::<BTreeSet<_>>();
    match values.len() {
        0 => fallback.to_string(),
        1 => values.iter().next().expect("one value").to_string(),
        _ => format!("mixed_{key}"),
    }
}

fn saturation_status(worker_count: usize, task_count: usize, queue_depth: usize) -> String {
    if task_count == 0 {
        "no_scheduler_tasks_started".to_string()
    } else if worker_count == 0 {
        "scheduler_worker_count_missing".to_string()
    } else if queue_depth > worker_count {
        "queued_parallel_work".to_string()
    } else {
        "within_worker_capacity".to_string()
    }
}
