use super::process_group::{
    compile_probe, group_and_descendants_absent, read_process_record, wait_for_natural_test_cleanup,
};
use super::{FixtureCaptureAdapter, root, spec};
use crate::fixture_scheduler::*;
use std::ffi::OsString;

#[test]
fn actual_adapter_contains_rapidly_reparented_setsid_descendants() {
    let root = root("detached-setsid");
    let probe = compile_probe(&root.join("build"));
    let fixture = spec("detached-setsid", ExpectedOutcome::pass(2));
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
        [
            "detached-direct-exit",
            record_path.to_str().unwrap(),
            "3",
            "900",
        ]
        .into_iter()
        .map(OsString::from)
        .collect(),
        4096,
        Vec::new(),
    )
    .unwrap();
    let run = scheduler.run(&lease).unwrap();
    let outcome = adapter
        .execute(&run.fixture, &run.lease, &run.environment)
        .unwrap();
    assert_eq!(outcome.verdict, OutcomeVerdict::Pass);
    let record = read_process_record(&record_path);
    assert!(record.descendants.is_empty());
    assert_eq!(record.denied, 3);
    let escaped = !group_and_descendants_absent(&record);
    if escaped {
        wait_for_natural_test_cleanup(&record);
    }
    assert!(
        !escaped,
        "adapter returned while detached descendants {:?} remained alive",
        record.descendants
    );
    scheduler.recover(&lease).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
