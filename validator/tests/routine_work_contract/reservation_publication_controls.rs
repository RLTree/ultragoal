use std::fs;
use std::os::unix::fs::DirBuilderExt;

use super::routine_plan_fixture::{RoutinePlanFixture, authority_path, isolate_fixture_test};
use super::routine_work::{
    RoutineCancellation, RoutineReuseInput, mediate_public_routine_execution,
    set_test_publication_ambiguity_after, set_test_publication_refusal_after,
    test_last_spawn_group_absent,
};

#[test]
fn output_stage_publication_refusal_rolls_back_exact_created_scope() {
    if isolate_fixture_test(
        "reservation_publication_controls::output_stage_publication_refusal_rolls_back_exact_created_scope",
    ) {
        return;
    }
    let mut fixture = RoutinePlanFixture::new("output-stage-publication-refusal");
    fs::remove_dir(fixture.repo.root().join("target/routine")).unwrap();
    let prepared = fixture.prepare().unwrap();
    prepare_authority(&fixture);
    set_test_publication_ambiguity_after(0);
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
    let target = fixture.repo.root().join("target");
    assert!(!target.join("routine").exists());
    assert!(fs::read_dir(target).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".routine-output-")
    }));
    fixture.finish();
}

#[test]
fn launch_stage_publication_refusal_cleans_exact_staged_program() {
    if isolate_fixture_test(
        "reservation_publication_controls::launch_stage_publication_refusal_cleans_exact_staged_program",
    ) {
        return;
    }
    let mut fixture = RoutinePlanFixture::new("launch-stage-publication-refusal");
    let prepared = fixture.prepare().unwrap();
    prepare_authority(&fixture);
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
    assert_launch_root_empty(&fixture);
    fixture.finish();
}

#[test]
fn child_lease_publication_refusal_reaps_before_stage_cleanup() {
    if isolate_fixture_test(
        "reservation_publication_controls::child_lease_publication_refusal_reaps_before_stage_cleanup",
    ) {
        return;
    }
    let mut fixture = RoutinePlanFixture::new("child-lease-publication-refusal");
    let prepared = fixture.prepare().unwrap();
    prepare_authority(&fixture);
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
    assert_launch_root_empty(&fixture);
    fixture.finish();
}

fn assert_launch_root_empty(fixture: &RoutinePlanFixture) {
    let root = authority_path(fixture)
        .parent()
        .unwrap()
        .join(".routine-authority-launch");
    assert!(root.exists());
    assert_eq!(fs::read_dir(root).unwrap().count(), 0);
}

fn prepare_authority(fixture: &RoutinePlanFixture) {
    let mut builder = fs::DirBuilder::new();
    builder.mode(0o700);
    builder.create(authority_path(fixture)).unwrap();
}
