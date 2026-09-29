use super::*;
use crate::repository_fit::product_adapter::{
    AdapterErrorId, FitPlanScope, plan_target_for_scope, prepare_apply_request,
};

#[test]
pub(crate) fn routine_configuration_scope_applies_only_its_two_bound_files() {
    let fixture = Fixture::new("routine-configuration-scope");
    fixture.write("AGENTS.md", b"user-owned authority\n");
    fixture.write(".gitignore", b"/target/\n");
    fixture.write("operator-state.txt", b"preserve exactly\n");
    let context = fixture.context();
    let record = assert_zero_write(&fixture, || {
        plan_target_for_scope(&context, FitPlanScope::RoutineConfiguration).unwrap()
    });
    assert_eq!(record.scope, FitPlanScope::RoutineConfiguration);
    assert_eq!(record.desired.file_count, 2);
    assert_eq!(record.conflict_count(), 0);
    assert_eq!(record.mutation_count(), 2);
    assert!(record.local_state.is_none());
    assert!(record.plan.local_state.is_none());
    assert_eq!(
        record
            .plan
            .mutations
            .iter()
            .map(|mutation| mutation.path.as_str())
            .collect::<Vec<_>>(),
        ["config/routine-public.json", "config/routines.json"]
    );

    let mut widened = record.clone();
    widened.scope = FitPlanScope::CompleteRepository;
    let widened_bytes = widened.to_machine_bytes().unwrap();
    let rejection = assert_zero_write(&fixture, || {
        match prepare_apply_request(&context, &widened_bytes, widened.plan_sha256()) {
            Err(rejection) => rejection,
            Ok(_) => panic!("widened routine plan must not prepare"),
        }
    });
    assert_eq!(rejection.id(), AdapterErrorId::StalePlan);

    let bytes = record.to_machine_bytes().unwrap();
    let prepared = assert_zero_write(&fixture, || {
        prepare_apply_request(&context, &bytes, record.plan_sha256()).unwrap()
    });
    let verification = execute(&fixture, prepared).unwrap();
    assert!(verification.idempotent());
    assert_eq!(verification.matched_files(), 2);
    assert_eq!(
        fs::read(fixture.root.join("AGENTS.md")).unwrap(),
        b"user-owned authority\n"
    );
    assert_eq!(
        fs::read(fixture.root.join(".gitignore")).unwrap(),
        b"/target/\n"
    );
    assert_eq!(
        fs::read(fixture.root.join("operator-state.txt")).unwrap(),
        b"preserve exactly\n"
    );
    let repeated =
        plan_target_for_scope(&fixture.context(), FitPlanScope::RoutineConfiguration).unwrap();
    assert_eq!(repeated.mutation_count(), 0);
    assert_eq!(repeated.conflict_count(), 0);
    assert!(repeated.local_state.is_none());
}
