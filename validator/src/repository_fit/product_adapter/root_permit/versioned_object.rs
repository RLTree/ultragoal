use super::*;

#[cfg(unix)]
pub(crate) fn metadata_versioned_object(
    metadata: &fs::Metadata,
    payload_sha256: Option<String>,
) -> Result<VersionedProtectedObject, FitAdapterError> {
    Ok(VersionedProtectedObject {
        object: metadata_object(metadata, payload_sha256)?,
        change_version: ProtectedChangeVersion {
            ctime_seconds: metadata.ctime(),
            ctime_nanoseconds: metadata.ctime_nsec(),
        },
    })
}

pub(crate) fn same_versioned_attachment_contract(
    left: &VersionedProtectedObject,
    right: &VersionedProtectedObject,
) -> bool {
    left.change_version == right.change_version
        && same_attachment_contract(&left.object, &right.object)
}

#[cfg(unix)]
pub(crate) fn bounded_file_read(
    file: &mut File,
    expected: u64,
) -> Result<Vec<u8>, FitAdapterError> {
    let mut bytes = Vec::with_capacity(expected.min(1024 * 1024) as usize);
    Read::by_ref(file)
        .take(MAX_FENCE_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| adapter_error(AdapterErrorId::TargetUnavailable))?;
    if bytes.len() as u64 > MAX_FENCE_FILE_BYTES {
        return Err(adapter_error(AdapterErrorId::TargetUnavailable));
    }
    Ok(bytes)
}

#[cfg(unix)]
pub(crate) fn object_identity(
    metadata: &fs::Metadata,
) -> (u64, u64, u64, u32, u32, u32, u64, i64, i64) {
    (
        metadata.dev(),
        metadata.ino(),
        metadata.nlink(),
        metadata.uid(),
        metadata.gid(),
        metadata.mode(),
        metadata.len(),
        metadata.ctime(),
        metadata.ctime_nsec(),
    )
}

pub(crate) fn production_authority_identity(
    authority_id: String,
) -> Result<Arc<AuthorityIdentity>, FitAdapterError> {
    if !valid_digest(&authority_id) {
        return Err(adapter_error(AdapterErrorId::ApplyPermitInvalid));
    }
    Ok(Arc::new(AuthorityIdentity { authority_id }))
}

/// Performs every deterministic request, context, target, and protected-tree
/// refusal before the authority store may be opened or initialized.
pub(crate) fn prepare_production_permit(
    context: &LiveContext,
    request: &OpaqueFitApplyRequest,
) -> Result<PreparedProductionPermit, FitAdapterError> {
    let protected_prestate = capture_protected(
        context.worktree_root(),
        request,
        ProtectedCaptureBoundary::PermitIssuance,
    )?;
    revalidate_apply_request(context, request)?;
    let permit_target = capture_target_descriptor_chain(context.worktree_root(), request)?;
    let target_prestate = permit_target.snapshot.clone();
    revalidate_apply_request(context, request)?;
    let protected_recheck = capture_protected(
        context.worktree_root(),
        request,
        ProtectedCaptureBoundary::PermitIssuanceRecheck,
    )?;
    let target_recheck = capture_target(context.worktree_root(), request)?;
    if target_recheck != target_prestate || protected_recheck != protected_prestate {
        return Err(adapter_error(AdapterErrorId::StalePlan));
    }
    let binding = permit_binding(request, &target_prestate, &protected_prestate)?;
    Ok(PreparedProductionPermit {
        binding,
        target_prestate,
        target_chain: permit_target.chain,
        protected_prestate,
    })
}

pub(crate) fn prepare_recovery_managed_ancestor_contract(
    context: &LiveContext,
    request: &OpaqueFitApplyRequest,
) -> Result<ManagedAncestorContract, FitAdapterError> {
    revalidate_apply_request(context, request)?;
    let mut capture = capture_target_descriptor_chain(context.worktree_root(), request)?;
    let snapshot = capture.snapshot.clone();
    capture.chain.revalidate()?;
    revalidate_apply_request(context, request)?;
    let recheck = capture_target(context.worktree_root(), request)?;
    if recheck != snapshot {
        return Err(adapter_error(AdapterErrorId::StalePlan));
    }
    managed_ancestor_contract(&snapshot, request)
}

pub(crate) fn prepared_managed_ancestor_contract(
    prepared: &PreparedProductionPermit,
    request: &OpaqueFitApplyRequest,
) -> Result<ManagedAncestorContract, FitAdapterError> {
    managed_ancestor_contract(&prepared.target_prestate, request)
}

pub(crate) fn production_reservation_binding(
    prepared: &PreparedProductionPermit,
    authority: &Arc<AuthorityIdentity>,
    issued_tick: u64,
    expires_tick: u64,
    nonce_sha256: String,
) -> Result<ProductionReservationBinding, FitAdapterError> {
    if !valid_digest(&nonce_sha256)
        || expires_tick < issued_tick
        || expires_tick.saturating_sub(issued_tick) > MAX_PERMIT_LIFETIME
    {
        return Err(adapter_error(AdapterErrorId::ApplyPermitInvalid));
    }
    let binding_sha256 = digest(
        &serde_json::to_vec(&("repository-fit-production-binding-v1", &prepared.binding))
            .map_err(|_| adapter_error(AdapterErrorId::ProjectionFailed))?,
    );
    let semantic_effect_id = digest(
        &serde_json::to_vec(&(
            "repository-fit-production-semantic-effect-v1",
            &prepared.binding.context_id,
            &prepared.binding.candidate_id,
            &prepared.binding.repository_root_id,
            &prepared.binding.worktree_root_id,
            &prepared.binding.root_binding,
            &prepared.binding.plan_record_sha256,
            &prepared.binding.plan_sha256,
            &prepared.binding.accepted_plan_sha256,
            &prepared.binding.desired_state_sha256,
            &prepared.binding.source_authority_sha256,
            &prepared.binding.target_prestate_sha256,
            &prepared.binding.allowed_mutation_set_sha256,
            &prepared.binding.rollback_policy_sha256,
            &prepared.binding.protected_prestate_sha256,
        ))
        .map_err(|_| adapter_error(AdapterErrorId::ProjectionFailed))?,
    );
    let target_scope_id = digest(
        &serde_json::to_vec(&(
            "repository-fit-production-target-scope-v1",
            &prepared.binding.repository_root_id,
            &prepared.binding.worktree_root_id,
            &prepared.binding.root_binding,
        ))
        .map_err(|_| adapter_error(AdapterErrorId::ProjectionFailed))?,
    );
    let permit_id = permit_id(
        authority.id(),
        &prepared.binding,
        issued_tick,
        expires_tick,
        &nonce_sha256,
    )?;
    Ok(ProductionReservationBinding {
        binding_sha256,
        semantic_effect_id,
        target_scope_id,
        permit_id,
        nonce_sha256,
    })
}
