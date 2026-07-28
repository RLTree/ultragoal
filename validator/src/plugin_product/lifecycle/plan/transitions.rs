use super::super::model::{
    LifecycleAuthorization, LifecycleEffect, LifecycleError, LifecycleIntent, LifecycleRequest,
    LifecycleState, PackageAuthority,
};

pub(super) fn transition(
    observed: &LifecycleState,
    request: &LifecycleRequest,
) -> Result<(LifecycleState, Vec<LifecycleEffect>), LifecycleError> {
    match request.intent {
        LifecycleIntent::FreshInstall => fresh_install(observed, request),
        LifecycleIntent::MonotonicUpdate => monotonic_update(observed, request),
        LifecycleIntent::FailedUpdateRecovery => failed_update_recovery(observed, request),
        LifecycleIntent::AuthorizedRollback => authorized_rollback(observed, request),
        LifecycleIntent::IdempotentReinstall => idempotent_reinstall(observed, request),
        LifecycleIntent::UninstallTeardown => uninstall_teardown(observed, request),
        LifecycleIntent::StaleCacheRecovery => stale_cache_recovery(observed, request),
        LifecycleIntent::RepeatUse => repeat_use(observed, request),
    }
}

fn fresh_install(
    observed: &LifecycleState,
    request: &LifecycleRequest,
) -> Result<(LifecycleState, Vec<LifecycleEffect>), LifecycleError> {
    require_write(&request.authorization)?;
    if observed.installed.is_some() || observed.cache.is_some() {
        return Err(LifecycleError::InvalidTransition);
    }
    let target = required_target(request)?;
    Ok((
        installed_state(observed, target),
        vec![
            LifecycleEffect::InstallPackage,
            LifecycleEffect::RefreshCache,
            LifecycleEffect::ProbeRuntime,
        ],
    ))
}

fn monotonic_update(
    observed: &LifecycleState,
    request: &LifecycleRequest,
) -> Result<(LifecycleState, Vec<LifecycleEffect>), LifecycleError> {
    require_write(&request.authorization)?;
    let current = current(observed)?;
    let target = required_target(request)?;
    if target.version <= current.version || target.package_sha256 == current.package_sha256 {
        return Err(LifecycleError::InvalidTransition);
    }
    Ok(update_effects(installed_state(observed, target)))
}

fn failed_update_recovery(
    observed: &LifecycleState,
    request: &LifecycleRequest,
) -> Result<(LifecycleState, Vec<LifecycleEffect>), LifecycleError> {
    require_write(&request.authorization)?;
    if !observed.recovery_required {
        return Err(LifecycleError::InvalidTransition);
    }
    let prior = request
        .prior_authority
        .clone()
        .filter(|state| state.installed.is_some())
        .ok_or(LifecycleError::MissingPriorAuthority)?;
    Ok((
        LifecycleState {
            generation: observed.generation.saturating_add(1),
            recovery_required: false,
            ..prior
        },
        vec![
            LifecycleEffect::RestorePriorAuthority,
            LifecycleEffect::ProbeRuntime,
        ],
    ))
}

fn authorized_rollback(
    observed: &LifecycleState,
    request: &LifecycleRequest,
) -> Result<(LifecycleState, Vec<LifecycleEffect>), LifecycleError> {
    require_write(&request.authorization)?;
    if !request.authorization.allow_downgrade {
        return Err(LifecycleError::DowngradeAuthorizationRequired);
    }
    let current = current(observed)?;
    let target = required_target(request)?;
    if target.version >= current.version || target.package_sha256 == current.package_sha256 {
        return Err(LifecycleError::InvalidTransition);
    }
    Ok(update_effects(installed_state(observed, target)))
}

fn idempotent_reinstall(
    observed: &LifecycleState,
    request: &LifecycleRequest,
) -> Result<(LifecycleState, Vec<LifecycleEffect>), LifecycleError> {
    let current = current(observed)?;
    let target = required_target(request)?;
    if &target != current || observed.cache.as_ref() != Some(current) {
        return Err(LifecycleError::InvalidTransition);
    }
    Ok(read_effects(observed.clone()))
}

fn uninstall_teardown(
    observed: &LifecycleState,
    request: &LifecycleRequest,
) -> Result<(LifecycleState, Vec<LifecycleEffect>), LifecycleError> {
    require_write(&request.authorization)?;
    current(observed)?;
    Ok((
        LifecycleState {
            generation: observed.generation.saturating_add(1),
            ..LifecycleState::default()
        },
        vec![
            LifecycleEffect::RemoveInstalledPackage,
            LifecycleEffect::RemoveCache,
            LifecycleEffect::VerifyTeardown,
        ],
    ))
}

fn stale_cache_recovery(
    observed: &LifecycleState,
    request: &LifecycleRequest,
) -> Result<(LifecycleState, Vec<LifecycleEffect>), LifecycleError> {
    require_write(&request.authorization)?;
    let current = current(observed)?.clone();
    if observed.cache.as_ref() == Some(&current) {
        return Err(LifecycleError::InvalidTransition);
    }
    Ok((
        installed_state(observed, current),
        vec![LifecycleEffect::RefreshCache, LifecycleEffect::ProbeRuntime],
    ))
}

fn repeat_use(
    observed: &LifecycleState,
    request: &LifecycleRequest,
) -> Result<(LifecycleState, Vec<LifecycleEffect>), LifecycleError> {
    let current = current(observed)?;
    if observed.cache.as_ref() != Some(current) {
        return Err(LifecycleError::InvalidTransition);
    }
    if request
        .target
        .as_ref()
        .is_some_and(|target| target != current)
    {
        return Err(LifecycleError::ExpectedPriorMismatch);
    }
    Ok(read_effects(observed.clone()))
}

fn update_effects(state: LifecycleState) -> (LifecycleState, Vec<LifecycleEffect>) {
    (
        state,
        vec![
            LifecycleEffect::InstallPackage,
            LifecycleEffect::RefreshCache,
            LifecycleEffect::ProbeRuntime,
        ],
    )
}

fn read_effects(state: LifecycleState) -> (LifecycleState, Vec<LifecycleEffect>) {
    (
        state,
        vec![
            LifecycleEffect::VerifyInstalledBytes,
            LifecycleEffect::ProbeRuntime,
        ],
    )
}

pub(super) fn writes_host_state(effects: &[LifecycleEffect]) -> bool {
    effects.iter().any(|effect| {
        matches!(
            effect,
            LifecycleEffect::InstallPackage
                | LifecycleEffect::RefreshCache
                | LifecycleEffect::RestorePriorAuthority
                | LifecycleEffect::RemoveInstalledPackage
                | LifecycleEffect::RemoveCache
        )
    })
}

fn installed_state(observed: &LifecycleState, target: PackageAuthority) -> LifecycleState {
    LifecycleState {
        installed: Some(target.clone()),
        cache: Some(target),
        generation: observed.generation.saturating_add(1),
        recovery_required: false,
    }
}

fn current(observed: &LifecycleState) -> Result<&PackageAuthority, LifecycleError> {
    observed
        .installed
        .as_ref()
        .ok_or(LifecycleError::InvalidTransition)
}

fn required_target(request: &LifecycleRequest) -> Result<PackageAuthority, LifecycleError> {
    request
        .target
        .clone()
        .ok_or(LifecycleError::InvalidTransition)
}

fn require_write(authorization: &LifecycleAuthorization) -> Result<(), LifecycleError> {
    authorization
        .allow_host_write
        .then_some(())
        .ok_or(LifecycleError::AuthorizationRequired)
}

pub(super) fn enforce_expected_prior(
    observed: &LifecycleState,
    authorization: &LifecycleAuthorization,
) -> Result<(), LifecycleError> {
    if let Some(expected) = &authorization.expected_installed_sha256
        && observed.installed.as_ref().map(|item| &item.package_sha256) != Some(expected)
    {
        return Err(LifecycleError::ExpectedPriorMismatch);
    }
    Ok(())
}
