use super::super::root_permit::ProductionPermitActivation;
use super::*;

#[cfg(target_vendor = "apple")]
pub(crate) fn execute_supported(
    context: &LiveContext,
    prepared: PreparedFitApply,
    recovery_intent: RepositoryFitRecoveryIntent,
    clock: &impl RepositoryFitTrustedClock,
    store: &impl RepositoryFitAuthorityStore,
    nonce: RepositoryFitApplyNonce,
    request_id: String,
) -> RepositoryFitProductionOutcome {
    let draft = match prepare_production_permit(context, prepared.request()) {
        Ok(draft) => draft,
        Err(error) => return RepositoryFitProductionOutcome::refusal(request_id, error),
    };
    let prepared_ancestors = match prepared_managed_ancestor_contract(&draft, prepared.request()) {
        Ok(contract) => contract,
        Err(error) => return RepositoryFitProductionOutcome::refusal(request_id, error),
    };
    let expected_recovery = match recovery_target_spec(prepared.request(), prepared_ancestors) {
        Ok(recovery) => recovery,
        Err(error) => return RepositoryFitProductionOutcome::refusal(request_id, error),
    };
    if recovery_intent.recovery() != &expected_recovery {
        return RepositoryFitProductionOutcome::refusal(
            request_id,
            adapter_error(AdapterErrorId::ApplyPermitInvalid),
        );
    }
    let issued_tick = match clock.trusted_tick() {
        Ok(tick) => tick,
        Err(_) => {
            return RepositoryFitProductionOutcome::refusal(
                request_id,
                adapter_error(AdapterErrorId::ApplyPermitExpired),
            );
        }
    };
    let expires_tick = match issued_tick.checked_add(DEFAULT_PERMIT_LIFETIME) {
        Some(tick) if tick.saturating_sub(issued_tick) <= MAX_PERMIT_LIFETIME => tick,
        _ => {
            return RepositoryFitProductionOutcome::refusal(
                request_id,
                adapter_error(AdapterErrorId::ApplyPermitExpired),
            );
        }
    };
    let execution_tick = match clock.trusted_tick() {
        Ok(tick) if tick >= issued_tick && tick <= expires_tick => tick,
        _ => {
            return RepositoryFitProductionOutcome::refusal(
                request_id,
                adapter_error(AdapterErrorId::ApplyPermitExpired),
            );
        }
    };
    let authority = match SealedProductionAuthority::open(store) {
        Ok(authority) => authority,
        Err(error) => {
            return RepositoryFitProductionOutcome::refusal(request_id, error.adapter_error());
        }
    };
    let nonce_sha256 = nonce.sha256();
    match authority.ledger.lookup_by_nonce(&nonce_sha256) {
        Ok(Some(existing)) => {
            return refuse_existing_execution(existing, &recovery_intent, request_id);
        }
        Ok(None) => {}
        Err(error) => {
            return RepositoryFitProductionOutcome::refusal(request_id, error.adapter_error());
        }
    }
    let binding = match production_reservation_binding(
        &draft,
        &authority.identity,
        issued_tick,
        expires_tick,
        nonce_sha256.clone(),
    ) {
        Ok(binding) => binding,
        Err(error) => return RepositoryFitProductionOutcome::refusal(request_id, error),
    };
    let reservation = authority.ledger.reserve(ReservationRequest {
        binding_sha256: binding.binding_sha256(),
        semantic_effect_id: binding.semantic_effect_id(),
        target_scope_id: binding.target_scope_id(),
        permit_id: binding.permit_id(),
        nonce_sha256: binding.nonce_sha256(),
        recovery_intent_sha256: recovery_intent.sha256(),
        issued_tick,
        expires_tick,
        recovery: recovery_intent.recovery(),
    });
    let token = match reservation {
        Ok(ReservationDecision::Acquired(token)) => token,
        Ok(ReservationDecision::Existing(existing)) => {
            return refuse_existing_execution(existing, &recovery_intent, request_id);
        }
        Err(error) => {
            return RepositoryFitProductionOutcome::refusal(request_id, error.adapter_error());
        }
    };
    test_after_reservation();
    let effects = match LocalEffects::open_with_mutation_grant(
        context.worktree_root(),
        prepared.request().unix_modes().clone(),
        LocalMutationGrant::issue(),
    ) {
        Ok(mut effects) => {
            test_configure_effects(&mut effects);
            effects
        }
        Err(error) => {
            let adapter = super::super::kernel_error(error);
            return settle_pre_effect_failure(
                &authority.ledger,
                token,
                request_id,
                adapter,
                execution_tick,
            );
        }
    };
    let request = prepared.into_request();
    let (permit, lease) = match activate_production_permit(
        context,
        &request,
        ProductionPermitActivation::new(
            draft,
            effects,
            authority.identity,
            issued_tick,
            expires_tick,
            nonce_sha256,
        ),
    ) {
        Ok(pair) => pair,
        Err(error) => {
            return settle_pre_effect_failure(
                &authority.ledger,
                token,
                request_id,
                error,
                execution_tick,
            );
        }
    };
    let effect_tick = match clock.trusted_tick() {
        Ok(tick) if tick >= execution_tick && tick <= expires_tick => tick,
        _ => {
            return settle_pre_effect_failure(
                &authority.ledger,
                token,
                request_id,
                adapter_error(AdapterErrorId::ApplyPermitExpired),
                execution_tick,
            );
        }
    };
    if !store.revalidate_protected_root() {
        return settle_pre_effect_failure(
            &authority.ledger,
            token,
            request_id,
            adapter_error(AdapterErrorId::ApplyPermitInvalid),
            effect_tick,
        );
    }
    let owner = match authority.ledger.begin_effect(token, effect_tick) {
        Ok(owner) => owner,
        Err(error) => {
            return RepositoryFitProductionOutcome::causal_refusal(
                request_id,
                error.adapter_error(),
                RepositoryFitLedgerState::Reserved,
                false,
            );
        }
    };
    test_after_effect_start_before_apply();
    let result = apply_with_root_permit(context, request, Some(permit), Some(lease), effect_tick);
    test_after_effect_before_terminal();
    match result {
        Ok(outcome) => settle_success(owner, request_id, outcome, effect_tick),
        Err(failure) => settle_failure(owner, request_id, failure, effect_tick),
    }
}
