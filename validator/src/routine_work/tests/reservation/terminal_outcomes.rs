use std::fs;
use std::os::unix::fs::DirBuilderExt;

use super::super::routine_plan_fixture::{
    RoutinePlanFixture, authority_path, isolate_fixture_test,
};
use super::super::routine_work::{
    RoutineCancellation, RoutineMediatorStatus, RoutineReuseInput, RoutineTerminalOutcome,
    mediate_public_routine_execution,
};

pub(super) fn cancelled_effect_keeps_a_cancelled_typed_terminal_outcome() {
    if isolate_fixture_test(
        "reservation::cancelled_effect_keeps_a_cancelled_typed_terminal_outcome",
    ) {
        return;
    }
    let mut fixture = RoutinePlanFixture::new("cancelled-terminal-outcome");
    let prepared = fixture.prepare().unwrap();
    create_private_routine_authority_directory(&fixture);
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

fn create_private_routine_authority_directory(fixture: &RoutinePlanFixture) {
    let mut builder = fs::DirBuilder::new();
    builder.mode(0o700);
    builder.create(authority_path(fixture)).unwrap();
}
