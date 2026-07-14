#[test]
fn already_exited_process_group_is_absent_after_adapter_return() {
    let root = root("already-exited");
    let probe = compile_probe(&root.join("build"));
    let fixture = spec("already-exited", ExpectedOutcome::pass(2));
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
    let adapter = FixtureCaptureAdapter::issue_test_native(
        &fixture,
        probe,
        ["exit-record", record_path.to_str().unwrap()]
            .into_iter()
            .map(OsString::from)
            .collect(),
        4096,
        Vec::new(),
    )
    .unwrap();
    let run = scheduler.run(&lease).unwrap();
    assert_eq!(
        adapter
            .execute(&run.fixture, &run.lease, &run.environment)
            .unwrap()
            .verdict,
        OutcomeVerdict::Pass
    );
    let record = read_process_record(&record_path);
    assert!(group_and_descendants_absent(&record));
    scheduler.recover(&lease).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
