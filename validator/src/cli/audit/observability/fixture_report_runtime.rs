use super::RuntimeFacts;

pub(super) fn telemetry(
    runtime: RuntimeFacts,
    context: RuntimeContext<'_>,
) -> crate::cli::observe::telemetry::RuntimeTelemetry {
    let worker_count = context
        .scheduler_metrics
        .iter()
        .map(|row| row.worker_count)
        .max()
        .unwrap_or(0);
    let task_count = context
        .scheduler_metrics
        .iter()
        .map(|row| row.task_count)
        .sum();
    let queue_depth = context
        .scheduler_metrics
        .iter()
        .map(|row| row.queue_depth)
        .sum();
    crate::cli::observe::telemetry::RuntimeTelemetry {
        duration_ms: runtime.elapsed_ms(),
        worker_count,
        task_count,
        queue_depth,
        cpu_ms: None,
        memory_bytes: None,
        io_bytes: None,
        cache_mode: context.cache_mode.to_string(),
        resource_measurement_status: context.resource_status.to_string(),
        retry_count: 0,
        backoff_ms: 0,
        saturation_status: saturation_status(&context, worker_count, task_count, queue_depth),
        repair_anchor_before: context.repair_anchor_before.to_string(),
        repair_anchor_after: "red_fixture_report_observability_emit".to_string(),
    }
}

pub(super) struct RuntimeContext<'a> {
    pub(super) cache_mode: &'a str,
    pub(super) resource_status: &'a str,
    pub(super) saturation_status: &'a str,
    pub(super) repair_anchor_before: &'a str,
    pub(super) scheduler_metrics: &'a [crate::scheduler::Metrics],
}

fn saturation_status(
    context: &RuntimeContext<'_>,
    worker_count: usize,
    task_count: usize,
    queue_depth: usize,
) -> String {
    if context.scheduler_metrics.is_empty() {
        return context.saturation_status.to_string();
    }
    if task_count == 0 {
        "red_report_no_fixture_tasks_started".to_string()
    } else if worker_count == 0 {
        "red_report_scheduler_worker_count_missing".to_string()
    } else if queue_depth > worker_count {
        "red_report_queued_parallel_work".to_string()
    } else {
        "red_report_within_worker_capacity".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{RuntimeContext, RuntimeFacts, telemetry};

    fn metrics(
        worker_count: usize,
        task_count: usize,
        queue_depth: usize,
    ) -> crate::scheduler::Metrics {
        crate::scheduler::Metrics {
            task_class: crate::scheduler::TaskClass::PureReadParallel.id(),
            worker_count,
            task_count,
            queue_depth,
            wall_ms: 1,
            cpu_ms: None,
            memory_bytes: None,
            io_bytes: None,
            cache_mode: "declared_local",
            resource_measurement_status: "test_scheduler_metrics",
            deterministic_ordering: true,
            shared_validation_artifact_writes_allowed: false,
        }
    }

    fn context<'a>(scheduler_metrics: &'a [crate::scheduler::Metrics]) -> RuntimeContext<'a> {
        RuntimeContext {
            cache_mode: "red_report_standalone_generate",
            resource_status: "runtime_test",
            saturation_status: "fallback_saturation",
            repair_anchor_before: "red_fixture_report_command_start",
            scheduler_metrics,
        }
    }

    #[test]
    fn red_report_runtime_uses_fallback_saturation_without_scheduler_metrics() {
        let runtime = telemetry(RuntimeFacts::from_elapsed_ms(9), context(&[]));
        assert_eq!(runtime.worker_count, 0);
        assert_eq!(runtime.task_count, 0);
        assert_eq!(runtime.queue_depth, 0);
        assert_eq!(runtime.saturation_status, "fallback_saturation");
    }

    #[test]
    fn red_report_runtime_projects_scheduler_saturation_states() {
        let no_tasks = [metrics(1, 0, 0)];
        assert_eq!(
            telemetry(RuntimeFacts::from_elapsed_ms(1), context(&no_tasks)).saturation_status,
            "red_report_no_fixture_tasks_started"
        );

        let no_workers = [metrics(0, 1, 0)];
        assert_eq!(
            telemetry(RuntimeFacts::from_elapsed_ms(1), context(&no_workers)).saturation_status,
            "red_report_scheduler_worker_count_missing"
        );

        let queued = [metrics(1, 2, 2)];
        assert_eq!(
            telemetry(RuntimeFacts::from_elapsed_ms(1), context(&queued)).saturation_status,
            "red_report_queued_parallel_work"
        );

        let within_capacity = [metrics(2, 2, 2)];
        assert_eq!(
            telemetry(RuntimeFacts::from_elapsed_ms(1), context(&within_capacity))
                .saturation_status,
            "red_report_within_worker_capacity"
        );
    }
}
