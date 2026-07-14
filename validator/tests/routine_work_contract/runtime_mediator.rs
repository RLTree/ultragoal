use std::collections::BTreeMap;

use super::routine_work::{
    PreparedRoutineExecution, RoutineAdapterSpec, RoutineInvocationSpec, prepare_routine_execution,
};
use super::runtime_adapter::{dirty_fixture, invocation_specs, preparation_error};

fn preparation_error_for(
    fixture: &super::runtime_adapter::AdapterFixture,
    first: RoutineInvocationSpec,
) -> &'static str {
    let mut invocations = invocation_specs(&fixture.context, &fixture.plan);
    invocations[0] = first;
    preparation_error(prepare_routine_execution(
        &fixture.context,
        &fixture.graph,
        &fixture.snapshot,
        &fixture.plan,
        RoutineAdapterSpec::new("routine", invocations),
    ))
    .cause()
}

#[test]
fn canonical_typed_binding_is_the_only_preparable_execution_shape() {
    let fixture = dirty_fixture("mediator-closed-binding");
    let invocations = invocation_specs(&fixture.context, &fixture.plan);
    assert!(invocations.iter().all(|invocation| {
        invocation.behavior_id == "rust-source-syntax-v1"
            && invocation.arguments == ["--json", "check", "routine"]
            && !invocation.read_sources.is_empty()
    }));
    assert!(matches!(
        prepare_routine_execution(
            &fixture.context,
            &fixture.graph,
            &fixture.snapshot,
            &fixture.plan,
            RoutineAdapterSpec::new("routine", invocations),
        )
        .unwrap(),
        PreparedRoutineExecution::Effect(_)
    ));
}

#[test]
fn argv_environment_program_and_unframed_substitutions_refuse_preparation() {
    let fixture = dirty_fixture("mediator-closed-binding-refusals");
    let invocation = || invocation_specs(&fixture.context, &fixture.plan).remove(0);

    assert_eq!(
        preparation_error_for(
            &fixture,
            invocation().test_with_arguments(vec!["--arbitrary".to_owned()]),
        ),
        "adapter-runner-binding-mismatch"
    );
    assert_eq!(
        preparation_error_for(
            &fixture,
            invocation().test_with_environment(BTreeMap::from([
                ("LANG".to_owned(), "C".to_owned()),
                ("LC_ALL".to_owned(), "C".to_owned()),
                ("PATH".to_owned(), "/arbitrary".to_owned()),
            ])),
        ),
        "adapter-runner-binding-mismatch"
    );
    assert_eq!(
        preparation_error_for(
            &fixture,
            invocation().test_with_program_path_hex("2f617262697472617279"),
        ),
        "adapter-runner-binding-mismatch"
    );
    assert_eq!(
        preparation_error_for(
            &fixture,
            invocation().test_with_program_sha256(format!("sha256:{}", "0".repeat(64))),
        ),
        "adapter-runner-binding-mismatch"
    );
    assert_eq!(
        preparation_error_for(
            &fixture,
            invocation().test_with_behavior_id("external-process-exit-v1"),
        ),
        "adapter-runner-binding-mismatch"
    );
    assert_eq!(
        preparation_error_for(&fixture, invocation().test_with_read_sources(Vec::new())),
        "adapter-runner-binding-mismatch"
    );
}
