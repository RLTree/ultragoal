use super::*;

pub(crate) fn prepare_apply_request(
    context: &LiveContext,
    plan_record_bytes: &[u8],
    accepted_plan_sha256: &str,
) -> Result<PreparedFitApply, FitAdapterError> {
    if plan_record_bytes.is_empty()
        || plan_record_bytes.len() > MAX_PLAN_RECORD_BYTES
        || !valid_digest(accepted_plan_sha256)
    {
        return Err(adapter_error(AdapterErrorId::InvalidPlanRecord));
    }
    let supplied: FitPlanRecord = serde_json::from_slice(plan_record_bytes)
        .map_err(|_| adapter_error(AdapterErrorId::InvalidPlanRecord))?;
    validate_record_constants(&supplied)?;
    let supplied_canonical = supplied.to_machine_bytes()?;
    if supplied_canonical != plan_record_bytes {
        return Err(adapter_error(AdapterErrorId::InvalidPlanRecord));
    }
    if supplied.plan.plan_sha256 != accepted_plan_sha256 {
        return Err(adapter_error(AdapterErrorId::AcceptanceMismatch));
    }

    let current = current_plan(context)?;
    let recomputed = plan_record(&current)?;
    if recomputed.to_machine_bytes()? != supplied_canonical {
        return Err(adapter_error(AdapterErrorId::StalePlan));
    }
    if !current.plan.conflicts.is_empty() {
        return Err(adapter_error(AdapterErrorId::PlanConflict));
    }
    context
        .revalidate()
        .map_err(|_| adapter_error(AdapterErrorId::ContextStale))?;
    let authorization = PlanAuthorization::new(
        current.target.context_id.clone(),
        current.target.candidate.candidate_id.clone(),
        current.plan.plan_sha256.clone(),
    )
    .map_err(kernel_error)?;
    let issuance = NEXT_APPLY_REQUEST_ISSUANCE
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
            value.checked_add(1)
        })
        .map_err(|_| adapter_error(AdapterErrorId::ProjectionFailed))?;
    let request_id = digest(
        &serde_json::to_vec(&(
            "repository-fit-opaque-apply-request-v2",
            &current.target.context_id,
            &current.target.candidate.candidate_id,
            &current.inspection.root_binding,
            &current.bundle.desired.state_sha256,
            &current.plan.plan_sha256,
            accepted_plan_sha256,
            &current.bundle.authority.authority_sha256,
            issuance,
        ))
        .map_err(|_| adapter_error(AdapterErrorId::ProjectionFailed))?,
    );
    #[cfg(test)]
    let projection = FitApplyPreparationProjection {
        schema_version: APPLY_PREPARATION_SCHEMA.to_owned(),
        request_id: request_id.clone(),
        context_id: current.target.context_id.clone(),
        candidate_id: current.target.candidate.candidate_id.clone(),
        root_binding: current.inspection.root_binding.clone(),
        desired_state_sha256: current.bundle.desired.state_sha256.clone(),
        plan_sha256: current.plan.plan_sha256.clone(),
        accepted_plan_sha256: accepted_plan_sha256.to_owned(),
        mutation_count: current.plan.all_mutations().len(),
        effect: "workspace_write_not_executed".to_owned(),
        claim_effect: CLAIM_EFFECT.to_owned(),
        support_limit: SUPPORT_LIMIT.to_owned(),
    };
    let request = OpaqueFitApplyRequest {
        seal: Arc::new(ApplyRequestSeal::new(
            issuance,
            request_seal_id(
                &request_id,
                &current.target.context_id,
                &current.target.candidate.candidate_id,
                issuance,
            ),
        )),
        request_id,
        context_id: current.target.context_id.clone(),
        candidate_id: current.target.candidate.candidate_id.clone(),
        root_binding: current.inspection.root_binding.clone(),
        accepted_plan_sha256: accepted_plan_sha256.to_owned(),
        plan_record_bytes: supplied_canonical,
        target: current.target,
        authority: current.bundle.authority,
        desired: current.bundle.desired,
        observed_modes: current.observed_modes,
        plan: current.plan,
        authorization,
        unix_modes: current.bundle.unix_modes,
    };
    Ok(PreparedFitApply {
        request,
        #[cfg(test)]
        projection,
    })
}

/// Rebuilds every authority-bearing adapter dimension from the live target.
/// The opaque request is accepted only when its exact internal plan, desired
/// bytes, source authority, modes, and canonical plan record still match.
pub(crate) fn revalidate_apply_request(
    context: &LiveContext,
    request: &OpaqueFitApplyRequest,
) -> Result<(), FitAdapterError> {
    context
        .revalidate()
        .map_err(|_| adapter_error(AdapterErrorId::ContextStale))?;
    if request.context_id != context.context_id()
        || !request.seal_matches(&request.seal_id())
        || request.accepted_plan_sha256 != request.plan.plan_sha256()
        || !request.authorization.matches(&request.plan)
    {
        return Err(adapter_error(AdapterErrorId::ApplyPermitInvalid));
    }

    let rebuilt_desired = DesiredState::new(
        request.desired.context_id.clone(),
        request.desired.candidate_id.clone(),
        request.desired.files.clone(),
    )
    .map_err(kernel_error)?;
    if rebuilt_desired.state_sha256 != request.desired.state_sha256 {
        return Err(adapter_error(AdapterErrorId::InvalidTemplateCatalog));
    }

    let current = current_plan(context)?;
    let current_record = plan_record(&current)?;
    let current_bytes = current_record.to_machine_bytes()?;
    let request_plan =
        plan_projection(&request.plan, &request.observed_modes, &request.unix_modes)?;
    if current_bytes != request.plan_record_bytes
        || request.target != current.target
        || request.authority != current.bundle.authority
        || request.desired.state_sha256 != current.bundle.desired.state_sha256
        || request.observed_modes != current.observed_modes
        || request.unix_modes != current.bundle.unix_modes
        || request_plan != current_record.plan
        || request.root_binding != current.inspection.root_binding
        || request.candidate_id != current.target.candidate.candidate_id
        || request.accepted_plan_sha256 != current.plan.plan_sha256
    {
        return Err(adapter_error(AdapterErrorId::StalePlan));
    }
    context
        .revalidate()
        .map_err(|_| adapter_error(AdapterErrorId::ContextStale))
}

pub(crate) fn request_seal_id(
    request_id: &str,
    context_id: &str,
    candidate_id: &str,
    issuance: u64,
) -> String {
    digest(
        &serde_json::to_vec(&(
            "repository-fit-opaque-apply-request-seal-v1",
            request_id,
            context_id,
            candidate_id,
            issuance,
        ))
        .expect("fixed request seal payload is serializable"),
    )
}

pub(crate) fn current_plan(context: &LiveContext) -> Result<CurrentPlan, FitAdapterError> {
    context
        .revalidate()
        .map_err(|_| adapter_error(AdapterErrorId::ContextStale))?;
    let target = target_projection(context)?;
    let mut bundle = compile(context)?;
    let mut reader = LocalRepository::open(context.worktree_root()).map_err(kernel_error)?;
    let mut inspection =
        inspect(mode(context), &bundle.desired, &mut reader).map_err(kernel_error)?;
    let observed_modes = bind_authoritative_modes(context, &bundle, &mut inspection)?;
    let mut mode_reader = LocalEffects::open(context.worktree_root(), bundle.unix_modes.clone())
        .map_err(kernel_error)?;
    let local_state_mode = mode_reader
        .read_unix_mode(
            &crate::repository_fit::CanonicalPath::parse(crate::repository_fit::LOCAL_STATE_PATH)
                .map_err(kernel_error)?,
        )
        .map_err(kernel_error)?;
    let local_state = inspect_local_state(&mut reader, local_state_mode).map_err(kernel_error)?;
    let mut observed_modes = observed_modes;
    observed_modes.insert(
        crate::repository_fit::LOCAL_STATE_PATH.to_owned(),
        local_state.observed_mode,
    );
    bundle.unix_modes.insert(
        crate::repository_fit::LOCAL_STATE_PATH.to_owned(),
        local_state.desired_mode,
    );
    let plan = plan_with_local_state(&inspection, &bundle.desired, Some(local_state.clone()))
        .map_err(kernel_error)?;
    context
        .revalidate()
        .map_err(|_| adapter_error(AdapterErrorId::ContextStale))?;
    Ok(CurrentPlan {
        target,
        bundle,
        inspection,
        observed_modes,
        local_state,
        plan,
    })
}
