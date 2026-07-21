pub(crate) fn recovery_state_after_completed_prefix(
    plan: &LifecyclePlan,
    completed_effects: &[LifecycleEffect],
) -> Result<LifecycleState, LifecycleError> {
    if !plan.effects.starts_with(completed_effects) {
        return Err(LifecycleError::InvalidTransition);
    }
    let mut state = plan.before.clone();
    let mut host_write_completed = false;
    for effect in completed_effects {
        match effect {
            LifecycleEffect::InstallPackage => {
                state.installed = plan.expected_after.installed.clone();
                host_write_completed = true;
            }
            LifecycleEffect::RefreshCache => {
                state.cache = plan.expected_after.cache.clone();
                host_write_completed = true;
            }
            LifecycleEffect::RestorePriorAuthority => {
                state.installed = plan.expected_after.installed.clone();
                state.cache = plan.expected_after.cache.clone();
                host_write_completed = true;
            }
            LifecycleEffect::RemoveInstalledPackage => {
                state.installed = None;
                host_write_completed = true;
            }
            LifecycleEffect::RemoveCache => {
                state.cache = None;
                host_write_completed = true;
            }
            LifecycleEffect::VerifyInstalledBytes
            | LifecycleEffect::VerifyTeardown
            | LifecycleEffect::ProbeRuntime => {}
        }
    }
    if host_write_completed {
        state.generation = plan.expected_after.generation;
    }
    state.recovery_required = true;
    state.validate()?;
    Ok(state)
}
