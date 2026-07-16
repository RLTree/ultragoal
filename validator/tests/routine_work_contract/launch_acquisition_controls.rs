use std::fs;
use std::os::unix::fs::DirBuilderExt;

use super::routine_plan_fixture::{RoutinePlanFixture, authority_path, isolate_fixture_test};
use super::routine_work::{
    RoutineCancellation, RoutineReuseInput, mediate_public_routine_execution,
    set_test_launch_cleanup_refusal, set_test_launch_panic_after_stat,
    set_test_launch_stat_failure_after,
};

#[test]
fn directory_stat_failures_carry_exact_launch_cleanup() {
    if isolate_fixture_test(
        "launch_acquisition_controls::directory_stat_failures_carry_exact_launch_cleanup",
    ) {
        return;
    }
    for observation in 0..2 {
        let mut fixture = RoutinePlanFixture::new(&format!("launch-stat-{observation}"));
        prepare_authority(&fixture);
        set_test_launch_stat_failure_after(observation);
        let error = execute(&fixture).unwrap_err();
        assert_eq!(
            error.cause(),
            "routine-production-launch-directory-stat-failed"
        );
        assert_launch_root_entries(&fixture, 0);
        let state = authority_state(&fixture);
        assert!(state.contains("\"state\":\"failed\""), "{state}");
        assert!(
            state.contains("\"staged_cleanup\":{\"status\":\"succeeded\"}"),
            "{state}"
        );
        fixture.finish();
    }
}

#[test]
fn launch_acquisition_panic_records_cleanup_before_resuming() {
    if isolate_fixture_test(
        "launch_acquisition_controls::launch_acquisition_panic_records_cleanup_before_resuming",
    ) {
        return;
    }
    let mut fixture = RoutinePlanFixture::new("launch-acquisition-panic");
    prepare_authority(&fixture);
    set_test_launch_panic_after_stat(0);
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = execute(&fixture);
    }))
    .unwrap_err();
    assert_eq!(
        panic.downcast_ref::<&str>().copied(),
        Some("routine-production-launch-acquisition-injected-panic")
    );
    assert_launch_root_entries(&fixture, 0);
    let state = authority_state(&fixture);
    assert!(state.contains("\"state\":\"failed\""), "{state}");
    assert!(
        state.contains("\"staged_cleanup\":{\"status\":\"succeeded\"}"),
        "{state}"
    );
    fixture.finish();
}

#[test]
fn output_rollback_cannot_substitute_for_launch_cleanup() {
    if isolate_fixture_test(
        "launch_acquisition_controls::output_rollback_cannot_substitute_for_launch_cleanup",
    ) {
        return;
    }
    let mut fixture = RoutinePlanFixture::new("launch-cleanup-refusal");
    prepare_authority(&fixture);
    set_test_launch_stat_failure_after(0);
    set_test_launch_cleanup_refusal(true);
    let error = execute(&fixture).unwrap_err();
    assert_eq!(
        error.cause(),
        "routine-production-launch-directory-stat-failed"
    );
    assert_launch_root_entries(&fixture, 1);
    let state = authority_state(&fixture);
    assert!(state.contains("\"state\":\"reserved\""), "{state}");
    assert!(
        state.contains("routine-production-launch-cleanup-injected-refusal"),
        "{state}"
    );
    assert!(!state.contains("\"state\":\"failed\""), "{state}");

    let retry = execute(&fixture).unwrap_err();
    assert_eq!(
        retry.cause(),
        "routine-production-session-continuity-required"
    );
    assert_launch_root_entries(&fixture, 1);
    remove_owned_empty_launch(&fixture);
    fixture.finish();
}

fn execute(
    fixture: &RoutinePlanFixture,
) -> Result<super::routine_work::RoutineMediationResult, super::routine_work::RoutineError> {
    let prepared = fixture.prepare().unwrap();
    mediate_public_routine_execution(
        Some(&authority_path(fixture)),
        &fixture.context,
        &fixture.plan,
        prepared,
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
}

fn prepare_authority(fixture: &RoutinePlanFixture) {
    let mut builder = fs::DirBuilder::new();
    builder.mode(0o700);
    builder.create(authority_path(fixture)).unwrap();
}

fn authority_state(fixture: &RoutinePlanFixture) -> String {
    fs::read_to_string(authority_path(fixture).join("routine-authority.state")).unwrap()
}

fn launch_root(fixture: &RoutinePlanFixture) -> std::path::PathBuf {
    authority_path(fixture)
        .parent()
        .unwrap()
        .join(".routine-authority-launch")
}

fn assert_launch_root_entries(fixture: &RoutinePlanFixture, expected: usize) {
    assert_eq!(
        fs::read_dir(launch_root(fixture)).unwrap().count(),
        expected
    );
}

fn remove_owned_empty_launch(fixture: &RoutinePlanFixture) {
    let entries = fs::read_dir(launch_root(fixture))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(entries.len(), 1);
    fs::remove_dir(entries[0].path()).unwrap();
}
