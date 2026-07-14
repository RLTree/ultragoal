fn actual_adapter_termination(
    label: &str,
    mode: &str,
    descendants: usize,
    trigger: TerminationTrigger,
) {
    let root = root(label);
    let probe = compile_probe(&root.join("build"));
    let wall_time = if matches!(trigger, TerminationTrigger::WallTime) {
        120
    } else {
        5_000
    };
    let fixture = timeout_spec(label, wall_time);
    let mut scheduler = FixtureScheduler::new(root.join("leases"));
    let lease = scheduler
        .schedule([fixture.clone()])
        .unwrap()
        .pop()
        .unwrap();
    let record_path = scheduler
        .run(&lease)
        .unwrap()
        .lease
        .root()
        .join("file/pids");
    let readiness_path = record_path.with_extension("ready");
    let output_limit = if matches!(trigger, TerminationTrigger::OutputLimit) {
        64
    } else {
        4096
    };
    let adapter = FixtureCaptureAdapter::issue_test_native(
        &fixture,
        probe,
        [
            mode,
            record_path.to_str().unwrap(),
            &descendants.to_string(),
            "1200",
            readiness_path.to_str().unwrap(),
        ]
        .into_iter()
        .map(OsString::from)
        .collect(),
        output_limit,
        Vec::new(),
    )
    .unwrap();
    let outcome = if matches!(trigger, TerminationTrigger::Interrupt) {
        std::thread::scope(|scope| {
            scope.spawn(|| {
                wait_for_readiness(&readiness_path);
                adapter.interrupt();
            });
            let run = scheduler.run(&lease).unwrap();
            adapter.execute(&run.fixture, &run.lease, &run.environment)
        })
    } else {
        let run = scheduler.run(&lease).unwrap();
        adapter.execute(&run.fixture, &run.lease, &run.environment)
    };
    assert_eq!(outcome.unwrap().verdict, OutcomeVerdict::Fail);
    let record = read_process_record(&record_path);
    assert!(record.descendants.is_empty());
    assert_eq!(record.denied, descendants);
    let escaped = !group_and_descendants_absent(&record);
    if escaped {
        wait_for_natural_test_cleanup(&record);
    }
    assert!(
        !escaped,
        "adapter returned before process group {} was absent",
        record.group
    );
    scheduler.recover(&lease).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn interrupt_before_readiness_is_deterministic_without_a_process_record() {
    let root = root("interrupt-before-readiness");
    let probe = compile_probe(&root.join("build"));
    let fixture = timeout_spec("interrupt-before-readiness", 5_000);
    let mut scheduler = FixtureScheduler::new(root.join("leases"));
    let lease = scheduler
        .schedule([fixture.clone()])
        .unwrap()
        .pop()
        .unwrap();
    let record_path = scheduler
        .run(&lease)
        .unwrap()
        .lease
        .root()
        .join("file/pids");
    let readiness_path = record_path.with_extension("ready");
    let adapter = FixtureCaptureAdapter::issue_test_native(
        &fixture,
        probe,
        [
            "delayed-readiness",
            record_path.to_str().unwrap(),
            readiness_path.to_str().unwrap(),
        ]
        .into_iter()
        .map(OsString::from)
        .collect(),
        4096,
        Vec::new(),
    )
    .unwrap();
    adapter.interrupt();
    let run = scheduler.run(&lease).unwrap();
    assert_eq!(
        adapter
            .execute(&run.fixture, &run.lease, &run.environment)
            .unwrap()
            .verdict,
        OutcomeVerdict::Fail
    );
    assert!(!record_path.exists());
    assert!(!readiness_path.exists());
    scheduler.recover(&lease).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn actual_wall_timeout_reaps_early_exiting_direct_child_after_spawn_is_denied() {
    actual_adapter_termination(
        "wall-direct-exit",
        "term-exit-descendants",
        1,
        TerminationTrigger::WallTime,
    );
}

#[test]
fn actual_wall_timeout_reaps_signal_resistant_direct_child_and_denies_multiple_spawns() {
    actual_adapter_termination(
        "wall-all-ignore",
        "all-ignore-descendants",
        3,
        TerminationTrigger::WallTime,
    );
}

#[test]
fn actual_interrupt_and_output_limit_reap_signal_resistant_groups_and_deny_spawns() {
    actual_adapter_termination(
        "interrupt-all-ignore",
        "all-ignore-descendants",
        2,
        TerminationTrigger::Interrupt,
    );
    actual_adapter_termination(
        "output-all-ignore",
        "output-limit-descendants",
        2,
        TerminationTrigger::OutputLimit,
    );
}
