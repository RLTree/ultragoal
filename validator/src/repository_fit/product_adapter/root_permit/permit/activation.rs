use super::*;

pub(crate) struct ProductionPermitActivation<E> {
    prepared: PreparedProductionPermit,
    effects: E,
    authority: Arc<AuthorityIdentity>,
    issued_tick: u64,
    expires_tick: u64,
    nonce_sha256: String,
}

impl<E> ProductionPermitActivation<E> {
    pub(crate) fn new(
        prepared: PreparedProductionPermit,
        effects: E,
        authority: Arc<AuthorityIdentity>,
        issued_tick: u64,
        expires_tick: u64,
        nonce_sha256: String,
    ) -> Self {
        Self {
            prepared,
            effects,
            authority,
            issued_tick,
            expires_tick,
            nonce_sha256,
        }
    }
}

/// Rechecks the complete pre-reservation state and moves the concrete effect
/// adapter into exactly one permit/lease pair. A post-reservation race can only
/// yield a terminal ledger rejection; it cannot reach the mutation kernel.
pub(crate) fn activate_production_permit<E: RepositoryFitPermitEffects>(
    context: &LiveContext,
    request: &OpaqueFitApplyRequest,
    activation: ProductionPermitActivation<E>,
) -> Result<(RepositoryFitApplyPermit, RepositoryFitMutationLease<E>), FitAdapterError> {
    let ProductionPermitActivation {
        mut prepared,
        mut effects,
        authority,
        issued_tick,
        expires_tick,
        nonce_sha256,
    } = activation;
    let reservation = production_reservation_binding(
        &prepared,
        &authority,
        issued_tick,
        expires_tick,
        nonce_sha256.clone(),
    )?;
    if reservation.binding_sha256
        != digest(
            &serde_json::to_vec(&("repository-fit-production-binding-v1", &prepared.binding))
                .map_err(|_| adapter_error(AdapterErrorId::ProjectionFailed))?,
        )
    {
        return Err(adapter_error(AdapterErrorId::ApplyPermitInvalid));
    }
    prepared
        .target_chain
        .revalidate()
        .map_err(|_| adapter_error(AdapterErrorId::StalePlan))?;
    revalidate_apply_request(context, request)?;
    require_root_binding(&mut effects, &request.root_binding)?;
    let effects_target = capture_target_descriptor_chain(context.worktree_root(), request)?;
    let protected_recheck = capture_protected(
        context.worktree_root(),
        request,
        ProtectedCaptureBoundary::PermitIssuanceRecheck,
    )?;
    let target_recheck = capture_target(context.worktree_root(), request)?;
    if effects_target.snapshot != prepared.target_prestate
        || target_recheck != prepared.target_prestate
        || effects_target.snapshot != target_recheck
        || protected_recheck != prepared.protected_prestate
    {
        return Err(adapter_error(AdapterErrorId::StalePlan));
    }
    let lease = RepositoryFitMutationLease {
        binding_id: prepared.binding.plan_record_sha256.clone(),
        authority: Arc::clone(&authority),
        seal: Arc::clone(&request.seal),
        effects: ScopedEffects::new(
            effects,
            request,
            context.worktree_root(),
            prepared.target_prestate.clone(),
            effects_target,
        ),
    };
    Ok((
        RepositoryFitApplyPermit {
            permit_id: reservation.permit_id,
            binding: prepared.binding,
            issued_tick,
            expires_tick,
            nonce_sha256,
            authority,
            seal: Arc::clone(&request.seal),
            target_prestate: prepared.target_prestate,
            target_chain: prepared.target_chain,
            protected_prestate: prepared.protected_prestate,
        },
        lease,
    ))
}

#[cfg(test)]
pub(crate) struct TestRepositoryFitPermitAuthority {
    pub(crate) identity: Arc<AuthorityIdentity>,
    pub(crate) reservations: Mutex<BTreeSet<String>>,
}

#[cfg(test)]
impl TestRepositoryFitPermitAuthority {
    pub(crate) fn new(secret: &[u8]) -> Result<Self, FitAdapterError> {
        if secret.len() < 32 {
            return Err(adapter_error(AdapterErrorId::ApplyPermitInvalid));
        }
        let authority_id = digest(
            &serde_json::to_vec(&(AUTHORITY_DOMAIN, digest(secret)))
                .map_err(|_| adapter_error(AdapterErrorId::ProjectionFailed))?,
        );
        Ok(Self {
            identity: Arc::new(AuthorityIdentity { authority_id }),
            reservations: Mutex::new(BTreeSet::new()),
        })
    }

    pub(crate) fn issue<E: RepositoryFitPermitEffects>(
        &self,
        context: &LiveContext,
        request: &OpaqueFitApplyRequest,
        mut effects: E,
        issued_tick: u64,
        expires_tick: u64,
        nonce: &[u8],
    ) -> Result<(RepositoryFitApplyPermit, RepositoryFitMutationLease<E>), FitAdapterError> {
        if nonce.len() < MIN_NONCE_BYTES
            || expires_tick < issued_tick
            || expires_tick.saturating_sub(issued_tick) > MAX_PERMIT_LIFETIME
        {
            return Err(adapter_error(AdapterErrorId::ApplyPermitInvalid));
        }
        let protected_prestate = capture_protected(
            context.worktree_root(),
            request,
            ProtectedCaptureBoundary::PermitIssuance,
        )?;
        revalidate_apply_request(context, request)?;
        require_root_binding(&mut effects, &request.root_binding)?;
        let permit_target = capture_target_descriptor_chain(context.worktree_root(), request)?;
        let target_prestate = permit_target.snapshot.clone();
        revalidate_apply_request(context, request)?;
        require_root_binding(&mut effects, &request.root_binding)?;
        let effects_target = capture_target_descriptor_chain(context.worktree_root(), request)?;
        let protected_recheck = capture_protected(
            context.worktree_root(),
            request,
            ProtectedCaptureBoundary::PermitIssuanceRecheck,
        )?;
        let target_recheck = capture_target(context.worktree_root(), request)?;
        if effects_target.snapshot != target_prestate
            || target_recheck != target_prestate
            || effects_target.snapshot != target_recheck
            || protected_recheck != protected_prestate
        {
            return Err(adapter_error(AdapterErrorId::StalePlan));
        }
        let binding = permit_binding(request, &target_prestate, &protected_prestate)?;
        let nonce_sha256 = digest(nonce);
        let reservation = digest(
            &serde_json::to_vec(&(
                &self.identity.authority_id,
                &binding.request_id,
                &binding.plan_record_sha256,
                request.seal.issuance(),
            ))
            .map_err(|_| adapter_error(AdapterErrorId::ProjectionFailed))?,
        );
        let nonce_reservation = format!("nonce:{nonce_sha256}");
        let effect_reservation = format!("effect:{reservation}");
        let mut reservations = self
            .reservations
            .lock()
            .map_err(|_| adapter_error(AdapterErrorId::ApplyPermitInvalid))?;
        if reservations.contains(&nonce_reservation) || reservations.contains(&effect_reservation) {
            return Err(adapter_error(AdapterErrorId::ApplyPermitReplayed));
        }
        reservations.insert(nonce_reservation);
        reservations.insert(effect_reservation);
        drop(reservations);
        let permit_id = permit_id(
            &self.identity.authority_id,
            &binding,
            issued_tick,
            expires_tick,
            &nonce_sha256,
        )?;
        let lease = RepositoryFitMutationLease {
            binding_id: binding.plan_record_sha256.clone(),
            authority: Arc::clone(&self.identity),
            seal: Arc::clone(&request.seal),
            effects: ScopedEffects::new(
                effects,
                request,
                context.worktree_root(),
                target_prestate.clone(),
                effects_target,
            ),
        };
        Ok((
            RepositoryFitApplyPermit {
                permit_id,
                binding,
                issued_tick,
                expires_tick,
                nonce_sha256,
                authority: Arc::clone(&self.identity),
                seal: Arc::clone(&request.seal),
                target_prestate,
                target_chain: permit_target.chain,
                protected_prestate,
            },
            lease,
        ))
    }
}
