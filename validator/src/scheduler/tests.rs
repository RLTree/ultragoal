use super::{SchedulerConfig, TaskClass, run_ordered};

#[test]
fn scheduler_rejects_unbounded_worker_requests() {
    assert!(SchedulerConfig::from_jobs(Some(0)).is_err());
    assert!(SchedulerConfig::from_jobs(Some(257)).is_err());
    assert_eq!(SchedulerConfig::from_jobs(Some(2)).unwrap().jobs(), 2);
}

#[test]
fn scheduler_returns_deterministic_order_after_parallel_join() {
    let config = SchedulerConfig::from_jobs(Some(4)).unwrap();
    let scheduled = run_ordered(
        config,
        TaskClass::PureReadParallel,
        (0usize..16)
            .rev()
            .map(|value| Box::new(move || value) as Box<dyn FnOnce() -> usize + Send>)
            .collect(),
    );
    assert_eq!(
        scheduled.values,
        (0..16).rev().collect::<Vec<_>>(),
        "results keep task order, not worker completion order"
    );
    assert_eq!(scheduled.metrics.worker_count, 4);
    assert!(scheduled.metrics.deterministic_ordering);
    assert!(scheduled.metrics.artifacts_are_isolated);
    assert_eq!(
        scheduled.metrics.resource_measurement_status,
        "wall_time_only_cpu_memory_io_unavailable"
    );
    assert_eq!(scheduled.metrics.cpu_ms, None);
    assert_eq!(scheduled.metrics.memory_bytes, None);
    assert_eq!(scheduled.metrics.io_bytes, None);
}

#[test]
fn scheduler_keeps_authority_writes_serial() {
    let config = SchedulerConfig::from_jobs(Some(8)).unwrap();
    let scheduled = run_ordered(
        config,
        TaskClass::SharedAuthorityWriteSerial,
        vec![
            Box::new(|| 1usize),
            Box::new(|| 2usize),
            Box::new(|| 3usize),
        ],
    );
    assert_eq!(scheduled.values, vec![1, 2, 3]);
    assert_eq!(scheduled.metrics.worker_count, 1);
    assert_eq!(
        scheduled.metrics.task_class,
        TaskClass::SharedAuthorityWriteSerial.id()
    );
}

#[test]
fn scheduler_default_jobs_uses_available_parallelism_minus_one_or_one() {
    let expected = std::thread::available_parallelism()
        .map(|count| count.get().saturating_sub(1).max(1))
        .unwrap_or(1);
    assert_eq!(SchedulerConfig::default_jobs(), expected);
    assert_eq!(SchedulerConfig::from_jobs(None).unwrap().jobs(), expected);
}

#[test]
fn scheduler_task_class_ids_cover_parallel_and_serial_contract() {
    let classes = [
        TaskClass::PureReadParallel,
        TaskClass::IsolatedTempWriteParallel,
        TaskClass::ExternalLiveBoundedParallel,
        TaskClass::SharedAuthorityWriteSerial,
        TaskClass::DestructiveOrMutatingSerial,
    ]
    .map(TaskClass::id);
    assert_eq!(
        classes,
        [
            "pure_read_parallel",
            "isolated_temp_write_parallel",
            "external_live_bounded_parallel",
            "shared_authority_write_serial",
            "destructive_or_mutating_serial"
        ]
    );
}
