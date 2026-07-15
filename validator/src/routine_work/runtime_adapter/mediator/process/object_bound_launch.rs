use super::*;

pub(crate) fn spawn_exact_program(
    program: &PinnedExecutable,
    root: &RootAnchor,
    argv: &[String],
    environment: &BTreeMap<String, String>,
) -> Result<SpawnSetupGuard, RoutineError> {
    run_test_process_pre_spawn_hook();
    let mut setup = SpawnSetupGuard::new(spawn_suspended(program, root, argv, environment)?);
    run_test_process_post_spawn_hook();
    if let Err(error) = validate_loaded_executable(setup.child()?, program) {
        setup.terminate_suspended()?;
        return Err(error);
    }
    program.validate()?;
    Ok(setup)
}
