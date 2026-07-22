use super::*;

#[test]
pub(crate) fn local_state_scope_revalidates_through_production_authority() {
    let fixture = Fixture::new("local-state-production-authority");
    let context = fixture.context();
    let plan = plan_target_for_scope(&context, FitPlanScope::LocalState).unwrap();
    let prepared = prepare_apply_request(
        &context,
        &plan.to_machine_bytes().unwrap(),
        plan.plan_sha256(),
    )
    .unwrap();

    let outcome = execute(
        &fixture,
        &context,
        prepared,
        "local-state-production-authority",
    );

    assert_eq!(outcome.status(), "applied");
    assert!(outcome.effect_started());
    assert_eq!(
        fs::read(fixture.root.join(".gitignore")).unwrap(),
        b"validation_artifacts/\n"
    );
}
