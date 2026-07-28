use super::*;

#[test]
pub(crate) fn routine_configuration_scope_revalidates_through_production_authority() {
    let fixture = Fixture::new("routine-configuration-production-authority");
    let preserved_link = fixture.root.join(".git/release-evidence/link");
    fs::create_dir_all(preserved_link.parent().unwrap()).unwrap();
    symlink("outside-the-fit-target", &preserved_link).unwrap();
    let context = fixture.context();
    let plan = plan_target_for_scope(&context, FitPlanScope::RoutineConfiguration).unwrap();
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
        "routine-configuration-production-authority",
    );

    assert_eq!(outcome.status(), "applied");
    assert!(outcome.effect_started());
    assert!(fixture.root.join("config/routine-public.json").is_file());
    assert!(fixture.root.join("config/routines.json").is_file());
    assert_eq!(
        fs::read_link(preserved_link).unwrap(),
        std::path::PathBuf::from("outside-the-fit-target")
    );
}
