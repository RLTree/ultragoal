use std::fs;
use std::os::unix::fs::DirBuilderExt;

use super::super::routine_plan_fixture::{
    authority_path, isolate_fixture_test, RoutinePlanFixture,
};
use super::super::routine_work::{
    mediate_public_routine_execution, set_test_publication_refusal_after,
    test_last_spawn_group_absent, RoutineCancellation, RoutineReuseInput,
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
