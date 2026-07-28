use std::fs;
use std::os::unix::fs::DirBuilderExt;

use super::super::routine_plan_fixture::{
    RoutinePlanFixture, authority_path, isolate_fixture_test,
};
use super::super::routine_work::{
    RoutineCancellation, RoutineError, RoutineErrorId, RoutineMediatorStatus, RoutineReuseInput,
    mediate_public_routine_execution,
    mediate_public_routine_execution_with_reservation_publication,
    set_test_publication_refusal_after, test_last_spawn_group_absent,
};

pub(super) fn launch_stage_publication_refusal_cleans_exact_staged_program() {
    if isolate_fixture_test(
        "reservation::launch_stage_publication_refusal_cleans_exact_staged_program",
    ) {
        return;
    }
    let mut fixture = RoutinePlanFixture::new("launch-stage-publication-refusal");
    let prepared = fixture.prepare().unwrap();
    create_private_routine_authority_directory(&fixture);
    set_test_publication_refusal_after(1);
    let error = mediate_public_routine_execution(
        Some(&authority_path(&fixture)),
        &fixture.context,
        &fixture.plan,
        prepared,
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .unwrap_err();
    assert_eq!(
        error.cause(),
        "routine-production-authority-publish-precommit"
    );
    assert_routine_authority_launch_directory_empty(&fixture);
    fixture.finish();
}

pub(super) fn child_lease_publication_refusal_reaps_before_stage_cleanup() {
    if isolate_fixture_test(
        "reservation::child_lease_publication_refusal_reaps_before_stage_cleanup",
    ) {
        return;
    }
    let mut fixture = RoutinePlanFixture::new("child-lease-publication-refusal");
    let prepared = fixture.prepare().unwrap();
    create_private_routine_authority_directory(&fixture);
    set_test_publication_refusal_after(2);
    let error = mediate_public_routine_execution(
        Some(&authority_path(&fixture)),
        &fixture.context,
        &fixture.plan,
        prepared,
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .unwrap_err();
    assert_eq!(
        error.cause(),
        "routine-production-authority-publish-precommit"
    );
    assert!(test_last_spawn_group_absent().unwrap());
    assert_routine_authority_launch_directory_empty(&fixture);
    fixture.finish();
}

pub(super) fn host_reservation_publication_failure_settles_without_workspace_effect() {
    if isolate_fixture_test(
        "reservation::host_reservation_publication_failure_settles_without_workspace_effect",
    ) {
        return;
    }
    let mut fixture = RoutinePlanFixture::new("host-reservation-publication-failure");
    let prepared = fixture.prepare().unwrap();
    let before = fixture.repo.tree();
    create_private_routine_authority_directory(&fixture);
    let mut reject = reject_host_reservation_publication;
    let error = mediate_public_routine_execution_with_reservation_publication(
        &authority_path(&fixture),
        &fixture.context,
        &fixture.plan,
        prepared,
        &mut reject,
    )
    .unwrap_err();
    assert_eq!(error.cause(), "routine-host-reservation-checkpoint-failed");
    assert_eq!(fixture.repo.tree(), before);
    let state =
        fs::read_to_string(authority_path(&fixture).join("routine-authority.state")).unwrap();
    assert!(state.contains("\"state\":\"failed\""), "{state}");
    assert!(
        state.contains("\"disposition\":\"reserved-pending\""),
        "{state}"
    );

    let retry = fixture.prepare().unwrap();
    let retry = mediate_public_routine_execution(
        Some(&authority_path(&fixture)),
        &fixture.context,
        &fixture.plan,
        retry,
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .unwrap();
    assert_eq!(retry.status(), RoutineMediatorStatus::CompleteExecution);
    fixture.finish();
}

pub(super) fn ambiguous_host_reservation_publication_preserves_recovery_custody() {
    if isolate_fixture_test(
        "reservation::ambiguous_host_reservation_publication_preserves_recovery_custody",
    ) {
        return;
    }
    let mut fixture = RoutinePlanFixture::new("ambiguous-host-reservation-publication");
    let prepared = fixture.prepare().unwrap();
    let before = fixture.repo.tree();
    create_private_routine_authority_directory(&fixture);
    let mut reject = reject_ambiguous_host_reservation_publication;
    let error = mediate_public_routine_execution_with_reservation_publication(
        &authority_path(&fixture),
        &fixture.context,
        &fixture.plan,
        prepared,
        &mut reject,
    )
    .unwrap_err();
    assert_eq!(
        error.cause(),
        "routine-host-reservation-checkpoint-ambiguous"
    );
    assert_eq!(fixture.repo.tree(), before);
    let state =
        fs::read_to_string(authority_path(&fixture).join("routine-authority.state")).unwrap();
    assert!(state.contains("\"state\":\"reserved\""), "{state}");
    assert!(state.contains("\"failure_evidence\""), "{state}");
    fixture.finish();
}

fn reject_host_reservation_publication(
    _: &crate::routine_work::RoutineReservationPublication,
) -> Result<(), RoutineError> {
    Err(RoutineError::new(
        RoutineErrorId::ObservationFailed,
        "routine-host-reservation-checkpoint-failed",
        None,
    ))
}

fn reject_ambiguous_host_reservation_publication(
    _: &crate::routine_work::RoutineReservationPublication,
) -> Result<(), RoutineError> {
    Err(RoutineError::new(
        RoutineErrorId::ObservationFailed,
        "routine-host-reservation-checkpoint-ambiguous",
        None,
    ))
}

fn assert_routine_authority_launch_directory_empty(fixture: &RoutinePlanFixture) {
    let root = authority_path(fixture)
        .parent()
        .unwrap()
        .join(".routine-authority-launch");
    assert!(root.exists());
    assert_eq!(fs::read_dir(root).unwrap().count(), 0);
}

fn create_private_routine_authority_directory(fixture: &RoutinePlanFixture) {
    let mut builder = fs::DirBuilder::new();
    builder.mode(0o700);
    builder.create(authority_path(fixture)).unwrap();
}
