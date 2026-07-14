use super::*;

pub(crate) fn preflight<E: RepositoryFitPermitEffects>(
    context: &LiveContext,
    request: &OpaqueFitApplyRequest,
    permit: Option<&mut RepositoryFitApplyPermit>,
    lease: Option<&mut RepositoryFitMutationLease<E>>,
    now_tick: u64,
) -> Result<(), FitAdapterError> {
    let permit = permit.ok_or_else(|| adapter_error(AdapterErrorId::ApplyPermitMissing))?;
    let lease = lease.ok_or_else(|| adapter_error(AdapterErrorId::ApplyLeaseInvalid))?;
    if now_tick < permit.issued_tick || now_tick > permit.expires_tick {
        return Err(adapter_error(AdapterErrorId::ApplyPermitExpired));
    }
    let expected_binding =
        permit_binding(request, &permit.target_prestate, &permit.protected_prestate)?;
    let expected_permit_id = permit_id(
        &permit.authority.authority_id,
        &expected_binding,
        permit.issued_tick,
        permit.expires_tick,
        &permit.nonce_sha256,
    )?;
    if permit.binding != expected_binding
        || permit.permit_id != expected_permit_id
        || !Arc::ptr_eq(&request.seal, &permit.seal)
        || !request.seal_matches(&request.seal_id())
    {
        return Err(adapter_error(AdapterErrorId::ApplyPermitInvalid));
    }
    if !Arc::ptr_eq(&request.seal, &lease.seal)
        || !Arc::ptr_eq(&permit.authority, &lease.authority)
        || lease.binding_id != permit.binding.plan_record_sha256
    {
        return Err(adapter_error(AdapterErrorId::ApplyLeaseInvalid));
    }
    let protected_before = capture_protected(
        context.worktree_root(),
        request,
        ProtectedCaptureBoundary::ImmediatePreEffect,
    )?;
    if protected_before != permit.protected_prestate {
        return Err(adapter_error(AdapterErrorId::StalePlan));
    }
    permit
        .target_chain
        .revalidate()
        .map_err(|_| adapter_error(AdapterErrorId::StalePlan))?;
    revalidate_apply_request(context, request)?;
    require_root_binding(&mut lease.effects, &request.root_binding)?;
    let target = capture_target_descriptor_chain(context.worktree_root(), request)?.snapshot;
    context
        .revalidate()
        .map_err(|_| adapter_error(AdapterErrorId::ContextStale))?;
    require_root_binding(&mut lease.effects, &request.root_binding)?;
    let protected_after = capture_protected(
        context.worktree_root(),
        request,
        ProtectedCaptureBoundary::ImmediatePreEffectRecheck,
    )?;
    let target_after = capture_target(context.worktree_root(), request)?;
    if target != permit.target_prestate
        || target_after != permit.target_prestate
        || target != target_after
        || protected_after != permit.protected_prestate
        || protected_before != protected_after
    {
        return Err(adapter_error(AdapterErrorId::StalePlan));
    }
    Ok(())
}

pub(crate) fn postflight<E: RepositoryFitPermitEffects>(
    context: &LiveContext,
    request: &OpaqueFitApplyRequest,
    permit: &RepositoryFitApplyPermit,
    effects: &mut ScopedEffects<E>,
) -> Result<(FitVerification, TargetSnapshot), FitAdapterError> {
    test_before_postflight_observation();
    if effects.scope_violation() {
        return Err(adapter_error(AdapterErrorId::ApplyMutationScopeViolation));
    }
    let protected_before = capture_protected(
        context.worktree_root(),
        request,
        ProtectedCaptureBoundary::Postflight,
    )?;
    if protected_before != permit.protected_prestate {
        return Err(adapter_error(AdapterErrorId::ApplyOutcomeInvalid));
    }
    effects.revalidate_authorized_target()?;
    require_root_binding(effects, &request.root_binding)?;
    let verification = verify(&request.desired, effects).map_err(super::super::kernel_error)?;
    let target_poststate =
        require_desired_target(context.worktree_root(), request, permit, effects)?;
    revalidate_post_context(context)?;
    require_root_binding(effects, &request.root_binding)?;
    let protected_after = capture_protected(
        context.worktree_root(),
        request,
        ProtectedCaptureBoundary::PostflightRecheck,
    )?;
    if protected_after != permit.protected_prestate || protected_before != protected_after {
        return Err(adapter_error(AdapterErrorId::ApplyOutcomeInvalid));
    }
    let target_recheck = require_desired_target(context.worktree_root(), request, permit, effects)?;
    if target_recheck != target_poststate {
        return Err(adapter_error(AdapterErrorId::ApplyOutcomeInvalid));
    }
    Ok((verification, target_recheck))
}

/// Performs the final finite observation before success settlement. Equal
/// versioned protected observations bracket the first complete target capture,
/// and an equal complete target recheck follows the protected-after capture.
/// The protected and target stability intervals therefore overlap at one
/// common final-green linearization point.
pub(crate) fn finalize_green<E: RepositoryFitPermitEffects>(
    context: &LiveContext,
    request: &OpaqueFitApplyRequest,
    permit: &RepositoryFitApplyPermit,
    expected_target: &TargetSnapshot,
    effects: &mut ScopedEffects<E>,
) -> Result<TargetSnapshot, FitAdapterError> {
    if effects.scope_violation() {
        return Err(adapter_error(AdapterErrorId::ApplyMutationScopeViolation));
    }
    let protected_before = capture_protected(
        context.worktree_root(),
        request,
        ProtectedCaptureBoundary::FinalGreen,
    )?;
    if protected_before != permit.protected_prestate {
        return Err(adapter_error(AdapterErrorId::ApplyOutcomeInvalid));
    }
    effects.revalidate_authorized_target()?;
    revalidate_post_context(context)?;
    require_root_binding(effects, &request.root_binding)?;
    let final_target = require_desired_target(context.worktree_root(), request, permit, effects)?;
    let protected_after = capture_protected(
        context.worktree_root(),
        request,
        ProtectedCaptureBoundary::FinalGreenRecheck,
    )?;
    if protected_after != permit.protected_prestate || protected_before != protected_after {
        return Err(adapter_error(AdapterErrorId::ApplyOutcomeInvalid));
    }
    let final_target_recheck =
        require_desired_target(context.worktree_root(), request, permit, effects)?;
    if &final_target != expected_target
        || final_target_recheck != final_target
        || &final_target_recheck != expected_target
    {
        return Err(adapter_error(AdapterErrorId::ApplyOutcomeInvalid));
    }
    Ok(final_target_recheck)
}

pub(crate) fn terminal_after_started_failure<E: RepositoryFitPermitEffects>(
    context: &LiveContext,
    request: &OpaqueFitApplyRequest,
    permit: &RepositoryFitApplyPermit,
    effects: &mut ScopedEffects<E>,
    transaction: Option<crate::repository_fit::AppliedFit>,
    scope_violation: bool,
) -> Result<RepositoryFitApplyOutcome, RepositoryFitApplyFailure<E>> {
    let chain_ready = effects.revalidate_authorized_target().is_ok();
    let protected_ready = capture_protected(
        context.worktree_root(),
        request,
        ProtectedCaptureBoundary::Rollback,
    )
    .is_ok_and(|protected| protected == permit.protected_prestate);
    let rollback_result = if chain_ready && protected_ready {
        transaction.map_or(Ok(()), |transaction| rollback(transaction, effects))
    } else {
        Err(crate::repository_fit::error(FitErrorId::RollbackFailed))
    };
    let reconciled = rollback_result.is_ok() && reconcile_prior(context, request, permit, effects);
    if reconciled && request.seal.rolled_back().is_ok() {
        let error = if scope_violation {
            adapter_error(AdapterErrorId::ApplyMutationScopeViolation)
        } else {
            adapter_error(AdapterErrorId::ApplyRolledBack)
        };
        Err(RepositoryFitApplyFailure::Terminal(TerminalApplyFailure {
            error,
            effect_started: true,
            rollback_complete: true,
        }))
    } else {
        let _ = request.seal.ambiguous();
        terminal_ambiguous()
    }
}

pub(crate) fn terminal_ambiguous<E: RepositoryFitPermitEffects>()
-> Result<RepositoryFitApplyOutcome, RepositoryFitApplyFailure<E>> {
    Err(RepositoryFitApplyFailure::Terminal(TerminalApplyFailure {
        error: adapter_error(AdapterErrorId::ApplyOutcomeAmbiguous),
        effect_started: true,
        rollback_complete: false,
    }))
}
