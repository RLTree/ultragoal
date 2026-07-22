use super::*;

pub(crate) fn inspect_target(
    context: &LiveContext,
) -> Result<FitInspectProjection, FitAdapterError> {
    let current = current_plan(context, FitPlanScope::CompleteRepository)?;
    let local_state = required_local_state(&current)?.clone();
    let authority = current.bundle.authority.clone();
    let desired = desired_projection(&current.bundle.desired);
    let inspection = inspection_projection(
        &current.inspection,
        &current.observed_modes,
        &current.bundle.unix_modes,
        Some(&local_state),
    );
    context
        .revalidate()
        .map_err(|_| adapter_error(AdapterErrorId::ContextStale))?;
    Ok(FitInspectProjection {
        schema_version: INSPECT_SCHEMA.to_owned(),
        target: current.target,
        authority,
        desired,
        inspection,
        local_state: local_state_projection(&local_state),
        effect: "read".to_owned(),
        claim_effect: CLAIM_EFFECT.to_owned(),
        support_limit: SUPPORT_LIMIT.to_owned(),
    })
}

pub(crate) fn plan_target(context: &LiveContext) -> Result<FitPlanRecord, FitAdapterError> {
    plan_target_for_scope(context, FitPlanScope::CompleteRepository)
}

pub(crate) fn plan_target_for_scope(
    context: &LiveContext,
    scope: FitPlanScope,
) -> Result<FitPlanRecord, FitAdapterError> {
    let current = current_plan(context, scope)?;
    plan_record(&current)
}

pub(crate) fn verify_target(
    context: &LiveContext,
) -> Result<FitVerificationProjection, FitAdapterError> {
    let current = current_plan(context, FitPlanScope::CompleteRepository)?;
    let local_state = required_local_state(&current)?.clone();
    let mut reader = LocalRepository::open(context.worktree_root()).map_err(kernel_error)?;
    let inspection_projection = inspection_projection(
        &current.inspection,
        &current.observed_modes,
        &current.bundle.unix_modes,
        Some(&local_state),
    );
    let desired = desired_projection(&current.bundle.desired);
    let projection = if current.inspection.classification == RepositoryClass::AlreadyFitted
        && local_state.mutation.is_none()
    {
        let verification = verify(&current.bundle.desired, &mut reader).map_err(kernel_error)?;
        if verification.root_binding != current.inspection.root_binding {
            return Err(adapter_error(AdapterErrorId::ContextStale));
        }
        let byte_verification_sha256 = verification.verification_sha256;
        let verification_sha256 = digest(
            &serde_json::to_vec(&(
                "repository-fit-mode-bound-verification-v1",
                &byte_verification_sha256,
                &current.inspection.inspection_sha256,
                &current.bundle.authority.authority_sha256,
                &current.bundle.desired.state_sha256,
                &verification.root_binding,
                &local_state.desired_sha256(),
            ))
            .map_err(|_| adapter_error(AdapterErrorId::ProjectionFailed))?,
        );
        FitVerificationProjection {
            schema_version: VERIFY_SCHEMA.to_owned(),
            target: current.target,
            authority: current.bundle.authority,
            desired,
            local_state: local_state_projection(&local_state),
            root_binding: verification.root_binding,
            inspection_sha256: current.inspection.inspection_sha256,
            matched_files: verification.matched_files,
            idempotent: verification.idempotent,
            byte_verification_sha256: Some(byte_verification_sha256),
            verification_sha256: Some(verification_sha256),
            failure: None,
            effect: "read".to_owned(),
            claim_effect: CLAIM_EFFECT.to_owned(),
            support_limit: SUPPORT_LIMIT.to_owned(),
        }
    } else {
        let causal_files = inspection_projection
            .files
            .iter()
            .filter(|row| row.disposition != "matching")
            .cloned()
            .collect::<Vec<_>>();
        FitVerificationProjection {
            schema_version: VERIFY_SCHEMA.to_owned(),
            target: current.target,
            authority: current.bundle.authority,
            desired,
            local_state: local_state_projection(&local_state),
            root_binding: current.inspection.root_binding,
            inspection_sha256: current.inspection.inspection_sha256,
            matched_files: current
                .inspection
                .files
                .iter()
                .filter(|row| row.disposition == ObservedDisposition::Matching)
                .count(),
            idempotent: false,
            byte_verification_sha256: None,
            verification_sha256: None,
            failure: Some(VerificationFailureProjection {
                error_id: "HUFIT-011".to_owned(),
                classification: inspection_projection.classification,
                compatibility: inspection_projection.compatibility,
                causal_files,
            }),
            effect: "read".to_owned(),
            claim_effect: CLAIM_EFFECT.to_owned(),
            support_limit: SUPPORT_LIMIT.to_owned(),
        }
    };
    context
        .revalidate()
        .map_err(|_| adapter_error(AdapterErrorId::ContextStale))?;
    Ok(projection)
}

fn required_local_state(current: &CurrentPlan) -> Result<&LocalStatePlan, FitAdapterError> {
    current
        .local_state
        .as_ref()
        .ok_or_else(|| adapter_error(AdapterErrorId::ProjectionFailed))
}
