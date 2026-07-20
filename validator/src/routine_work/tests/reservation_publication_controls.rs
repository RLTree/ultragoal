use std::fs;
use std::os::unix::fs::DirBuilderExt;

use super::routine_plan_fixture::{RoutinePlanFixture, authority_path, isolate_fixture_test};
use super::routine_work::{
    RoutineCancellation, RoutineMediatorStatus, RoutineReuseInput, RoutineTerminalOutcome,
    mediate_public_routine_execution, set_test_publication_ambiguity_after,
    set_test_publication_refusal_after, test_last_spawn_group_absent,
};

#[test]
fn reservation_publication_ambiguity_is_durable_before_workspace_effects() {
    if isolate_fixture_test(
        "reservation_publication_controls::reservation_publication_ambiguity_is_durable_before_workspace_effects",
    ) {
        return;
    }
    let mut fixture = RoutinePlanFixture::new("reservation-publication-ambiguity");
    let prepared = fixture.prepare().unwrap();
    let before = fixture.repo.tree();
    prepare_authority(&fixture);
    set_test_publication_ambiguity_after(0);
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
        "routine-production-authority-publish-ambiguous"
    );
    assert_eq!(fixture.repo.tree(), before);
    let state =
        fs::read_to_string(authority_path(&fixture).join("routine-authority.state")).unwrap();
    assert!(state.contains("\"state\":\"ambiguous\""), "{state}");
    assert!(state.contains("\"cause\":\"reserve\""), "{state}");
    fixture.finish();
}

#[test]
fn cancelled_effect_keeps_a_cancelled_typed_terminal_outcome() {
    if isolate_fixture_test(
        "reservation_publication_controls::cancelled_effect_keeps_a_cancelled_typed_terminal_outcome",
    ) {
        return;
    }
    let mut fixture = RoutinePlanFixture::new("cancelled-terminal-outcome");
    let prepared = fixture.prepare().unwrap();
    prepare_authority(&fixture);
    let cancellation = RoutineCancellation::new();
    cancellation.cancel();

    let result = mediate_public_routine_execution(
        Some(&authority_path(&fixture)),
        &fixture.context,
        &fixture.plan,
        prepared,
        cancellation,
        RoutineReuseInput::default(),
    )
    .expect("cancelled routine settles durably");

    assert_eq!(result.status(), RoutineMediatorStatus::Cancelled);
    assert_eq!(
        result.terminal_outcome(),
        Some(RoutineTerminalOutcome::Cancelled)
    );
    let state =
        fs::read_to_string(authority_path(&fixture).join("routine-authority.state")).unwrap();
    assert!(state.contains("\"state\":\"cancelled\""), "{state}");
    fixture.finish();
}

#[test]
fn output_stage_publication_ambiguity_is_durable_and_rolls_back_exact_scope() {
    if isolate_fixture_test(
        "reservation_publication_controls::output_stage_publication_ambiguity_is_durable_and_rolls_back_exact_scope",
    ) {
        return;
    }
    let mut fixture = RoutinePlanFixture::new("output-stage-publication-refusal");
    fs::remove_dir(fixture.repo.root().join("target/routine")).unwrap();
    let prepared = fixture.prepare().unwrap();
    prepare_authority(&fixture);
    set_test_publication_ambiguity_after(1);
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
        "routine-production-authority-publish-ambiguous"
    );
    let state =
        fs::read_to_string(authority_path(&fixture).join("routine-authority.state")).unwrap();
    assert!(state.contains("\"state\":\"ambiguous\""), "{state}");
    assert!(state.contains("\"cause\":\"output-custody\""), "{state}");
    let target = fixture.repo.root().join("target");
    assert!(!target.join("routine").exists());
    assert!(fs::read_dir(target).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".routine-output-")
    }));
    let before_retry = fixture.repo.tree();
    let retry = fixture.prepare().unwrap();
    let error = mediate_public_routine_execution(
        Some(&authority_path(&fixture)),
        &fixture.context,
        &fixture.plan,
        retry,
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .unwrap_err();
    assert_eq!(
        error.cause(),
        "routine-production-session-continuity-required"
    );
    assert_eq!(fixture.repo.tree(), before_retry);
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

#[test]
fn terminal_publication_ambiguity_preserves_cleanup_and_refuses_replay() {
    if isolate_fixture_test(
        "reservation_publication_controls::terminal_publication_ambiguity_preserves_cleanup_and_refuses_replay",
    ) {
        return;
    }
    let mut fixture = RoutinePlanFixture::new("terminal-publication-ambiguity");
    let prepared = fixture.prepare().unwrap();
    prepare_authority(&fixture);
    set_test_publication_ambiguity_after(5);
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
        "routine-production-authority-publish-ambiguous"
    );
    let state =
        fs::read_to_string(authority_path(&fixture).join("routine-authority.state")).unwrap();
    assert!(state.contains("\"state\":\"ambiguous\""), "{state}");
    assert!(
        state.contains("\"cause\":\"terminal-settlement\""),
        "{state}"
    );
    assert!(state.contains("\"failure_evidence\""), "{state}");
    assert!(test_last_spawn_group_absent().unwrap());
    assert_launch_root_empty(&fixture);
    let retry = fixture.prepare().unwrap();
    let retry = mediate_public_routine_execution(
        Some(&authority_path(&fixture)),
        &fixture.context,
        &fixture.plan,
        retry,
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .unwrap_err();
    assert_eq!(
        retry.cause(),
        "routine-production-session-continuity-required"
    );
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
