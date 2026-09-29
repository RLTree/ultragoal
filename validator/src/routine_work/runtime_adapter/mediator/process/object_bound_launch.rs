use super::*;

pub(crate) fn spawn_exact_program(
    program: &PinnedExecutable,
    root: &RootAnchor,
    argv: &[String],
    environment: &BTreeMap<String, String>,
) -> Result<SpawnSetupGuard, RoutineError> {
    run_test_process_pre_spawn_hook();
    let setup = SpawnSetupGuard::new(spawn_suspended(program, root, argv, environment)?);
    let (setup, ()) = setup.configure(|setup| {
        run_test_process_post_spawn_hook();
        setup.validate_loaded(program)?;
        run_test_loaded_object_hook();
        program.validate()
    })?;
    Ok(setup)
}
