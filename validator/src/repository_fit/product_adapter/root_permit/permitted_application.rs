use super::*;

/// Consumes the exact opaque request and exclusive lease. Every refusal before
/// the seal transition returns both intact. Once the transition succeeds, no
/// path returns reusable mutation authority.
pub(crate) fn apply_with_root_permit<E: RepositoryFitPermitEffects>(
    context: &LiveContext,
    request: OpaqueFitApplyRequest,
    mut permit: Option<RepositoryFitApplyPermit>,
    mut lease: Option<RepositoryFitMutationLease<E>>,
    now_tick: u64,
) -> Result<RepositoryFitApplyOutcome, RepositoryFitApplyFailure<E>> {
    let preflight = preflight(context, &request, permit.as_mut(), lease.as_mut(), now_tick);
    if let Err(error) = preflight {
        return Err(RepositoryFitApplyFailure::PreEffect(Box::new(
            PreEffectFailure {
                error,
                request,
                permit,
                lease,
            },
        )));
    }
    let permit = permit.expect("preflight requires a permit");
    let mut lease = lease.expect("preflight requires a lease");
    if let Err(error) = request.seal.begin() {
        return Err(RepositoryFitApplyFailure::Terminal(TerminalApplyFailure {
            error,
            effect_started: true,
            rollback_complete: false,
        }));
    }

    let mutation_count = request.plan.mutations.len();
    match apply(&request.plan, &request.authorization, &mut lease.effects) {
        Ok(transaction) => {
            let postflight = postflight(context, &request, &permit, &mut lease.effects);
            match postflight {
                Ok((verification, target_poststate)) => {
                    test_before_final_green_observation();
                    let final_target = match finalize_green(
                        context,
                        &request,
                        &permit,
                        &target_poststate,
                        &mut lease.effects,
                    ) {
                        Ok(final_target) => final_target,
                        Err(_) => {
                            let scope_violation = lease.effects.scope_violation();
                            return terminal_after_started_failure(
                                context,
                                &request,
                                &permit,
                                &mut lease.effects,
                                Some(transaction),
                                scope_violation,
                            );
                        }
                    };
                    if request.seal.settle().is_err() {
                        return terminal_ambiguous();
                    }
                    Ok(success_outcome(
                        &request,
                        &permit,
                        &verification,
                        &final_target,
                        mutation_count,
                    ))
                }
                Err(_) => {
                    let scope_violation = lease.effects.scope_violation();
                    terminal_after_started_failure(
                        context,
                        &request,
                        &permit,
                        &mut lease.effects,
                        Some(transaction),
                        scope_violation,
                    )
                }
            }
        }
        Err(_) => {
            let scope_violation = lease.effects.scope_violation();
            terminal_after_started_failure(
                context,
                &request,
                &permit,
                &mut lease.effects,
                None,
                scope_violation,
            )
        }
    }
}

#[cfg(test)]
pub(crate) fn test_before_final_green_observation() {
    BEFORE_FINAL_GREEN_OBSERVATION.with(|slot| {
        if let Some(action) = slot.borrow_mut().take() {
            action();
        }
    });
}

#[cfg(test)]
pub(crate) fn test_before_postflight_observation() {
    BEFORE_POSTFLIGHT_OBSERVATION.with(|slot| {
        if let Some(action) = slot.borrow_mut().take() {
            action();
        }
    });
}

#[cfg(not(test))]
pub(crate) const fn test_before_postflight_observation() {}

#[cfg(not(test))]
pub(crate) const fn test_before_final_green_observation() {}

#[cfg(test)]
pub(crate) fn test_target_capture_point(phase: TargetCapturePhase, path: &str) {
    let action = TARGET_CAPTURE_HOOK.with(|slot| {
        let mut hook = slot.borrow_mut();
        if hook
            .as_ref()
            .is_some_and(|hook| hook.phase == phase && hook.path == path)
        {
            hook.take().map(|hook| hook.action)
        } else {
            None
        }
    });
    if let Some(action) = action {
        action();
    }
}

#[cfg(not(test))]
pub(crate) const fn test_target_capture_point(_phase: TargetCapturePhase, _path: &str) {}

#[cfg(test)]
pub(crate) fn test_protected_capture_point(
    boundary: ProtectedCaptureBoundary,
    phase: ProtectedCapturePhase,
    path: &[u8],
) {
    let action = PROTECTED_CAPTURE_HOOK.with(|slot| {
        let mut hook = slot.borrow_mut();
        if hook.as_ref().is_some_and(|hook| {
            hook.boundary == boundary && hook.phase == phase && hook.path == path
        }) {
            hook.take().map(|hook| hook.action)
        } else {
            None
        }
    });
    if let Some(action) = action {
        action();
    }
}

#[cfg(test)]
pub(crate) fn test_reconciliation_target_point(phase: ReconciliationTargetPhase) {
    let action = RECONCILIATION_TARGET_HOOK.with(|slot| {
        let mut hook = slot.borrow_mut();
        if hook.as_ref().is_some_and(|hook| hook.phase == phase) {
            hook.take().map(|hook| hook.action)
        } else {
            None
        }
    });
    if let Some(action) = action {
        action();
    }
}

#[cfg(not(test))]
pub(crate) const fn test_reconciliation_target_point(_phase: ReconciliationTargetPhase) {}

#[cfg(not(test))]
pub(crate) const fn test_protected_capture_point(
    _boundary: ProtectedCaptureBoundary,
    _phase: ProtectedCapturePhase,
    _path: &[u8],
) {
}
