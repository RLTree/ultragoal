use super::*;

pub(crate) fn reconcile_prior<E: RepositoryFitPermitEffects>(
    context: &LiveContext,
    request: &OpaqueFitApplyRequest,
    permit: &RepositoryFitApplyPermit,
    effects: &mut ScopedEffects<E>,
) -> bool {
    let protected_before = capture_protected(
        context.worktree_root(),
        request,
        ProtectedCaptureBoundary::AmbiguityReconciliation,
    );
    // Retain the exact snapshot recorded after the last mediated effect. Only
    // observations equal to this ctime-bound state may then use semantic
    // rollback equivalence against the permit prestate.
    let authorized_target = match effects.revalidate_authorized_target() {
        Ok(authorized_target) => authorized_target,
        Err(_) => return false,
    };
    test_reconciliation_target_point(ReconciliationTargetPhase::AuthorizedRevalidation);
    let target = capture_target(context.worktree_root(), request);
    test_reconciliation_target_point(ReconciliationTargetPhase::FirstTarget);
    let root = require_root_binding(effects, &request.root_binding);
    let context_valid = context.revalidate();
    let protected_after = capture_protected(
        context.worktree_root(),
        request,
        ProtectedCaptureBoundary::AmbiguityReconciliationRecheck,
    );
    test_reconciliation_target_point(ReconciliationTargetPhase::ProtectedAfter);
    let target_after = capture_target(context.worktree_root(), request);
    matches!((protected_before, target, root, context_valid, protected_after, target_after),
        (Ok(protected_before), Ok(target), Ok(()), Ok(()), Ok(protected_after), Ok(target_after))
            if target == authorized_target
                && target_after == authorized_target
                && target_rollback_equivalent(&authorized_target, &permit.target_prestate, request)
                && protected_before == permit.protected_prestate
                && protected_after == permit.protected_prestate
                && protected_before == protected_after)
}

pub(crate) fn success_outcome(
    request: &OpaqueFitApplyRequest,
    permit: &RepositoryFitApplyPermit,
    verification: &FitVerification,
    target_poststate: &TargetSnapshot,
    mutation_count: usize,
) -> RepositoryFitApplyOutcome {
    let status = if mutation_count == 0 {
        "idempotent"
    } else {
        "applied"
    };
    let outcome_id = digest(
        &serde_json::to_vec(&(
            "repository-fit-apply-outcome-v1",
            &request.request_id,
            &request.plan.plan_sha256,
            &request.desired.state_sha256,
            &verification.verification_sha256,
            &target_poststate.sha256,
            &permit.protected_prestate.sha256,
            mutation_count,
            request.plan.mutations.len(),
            status,
        ))
        .expect("fixed outcome payload is serializable"),
    );
    RepositoryFitApplyOutcome {
        schema_version: "RepositoryFitApplyOutcome-v1",
        outcome_id,
        request_id: request.request_id.clone(),
        plan_sha256: request.plan.plan_sha256.clone(),
        desired_state_sha256: request.desired.state_sha256.clone(),
        verification_sha256: verification.verification_sha256.clone(),
        target_poststate_sha256: target_poststate.sha256.clone(),
        mutation_count: request.plan.mutations.len(),
        status,
        effect: "workspace_write",
        claim_effect: "none",
        support_limit: "internal permit mediation only; root issuance and public apply dispatch absent",
    }
}

pub(crate) fn require_root_binding(
    effects: &mut impl FitReader,
    expected: &str,
) -> Result<(), FitAdapterError> {
    match effects.root_binding() {
        Ok(observed) if observed == expected => Ok(()),
        _ => Err(adapter_error(AdapterErrorId::ContextStale)),
    }
}

pub(crate) fn revalidate_post_context(context: &LiveContext) -> Result<(), FitAdapterError> {
    let mut request = BuildRequest::new(context.worktree_root())
        .expect_repository_root(PathBuf::from(&context.roots().repository_root))
        .expect_worktree_root(PathBuf::from(&context.roots().worktree_root));
    for (key, value) in &context.configuration().public_values {
        request = request.bind_non_secret_configuration(key.clone(), value.clone());
    }
    for source in &context.configuration().secret_sources {
        request = request.bind_secret_source(source.name.clone(), source.public_version.clone());
    }
    for input in context.selected_inputs() {
        request = request.select_input(input.relative_path.clone());
    }
    for tool in &context.capabilities().tools {
        request = request.probe_tool(tool.name.clone());
    }
    let current =
        LiveContext::build(request).map_err(|_| adapter_error(AdapterErrorId::ContextStale))?;
    let before = context.candidate();
    let after = current.candidate();
    if current.roots() != context.roots()
        || current.configuration() != context.configuration()
        || current.capabilities() != context.capabilities()
        || current.permissions() != context.permissions()
        || current.selected_inputs() != context.selected_inputs()
        || after.head_commit != before.head_commit
        || after.head_tree != before.head_tree
        || after.branch != before.branch
        || after.staged_diff_sha256 != before.staged_diff_sha256
    {
        return Err(adapter_error(AdapterErrorId::ContextStale));
    }
    Ok(())
}

pub(crate) fn permit_binding(
    request: &OpaqueFitApplyRequest,
    target: &TargetSnapshot,
    protected: &ProtectedSnapshot,
) -> Result<PermitBinding, FitAdapterError> {
    let mutations = request.all_mutations();
    let mutation_rows = mutations
        .iter()
        .map(|mutation| {
            Ok((
                mutation.path.as_str(),
                &mutation.expected,
                mutation.replacement_sha256(),
                mutation.prior.as_deref().map(digest),
                request
                    .unix_modes
                    .get(mutation.path.as_str())
                    .copied()
                    .ok_or_else(|| adapter_error(AdapterErrorId::ApplyPermitInvalid))?,
                request
                    .observed_modes
                    .get(mutation.path.as_str())
                    .copied()
                    .ok_or_else(|| adapter_error(AdapterErrorId::ApplyPermitInvalid))?,
            ))
        })
        .collect::<Result<Vec<_>, FitAdapterError>>()?;
    let allowed_mutation_set_sha256 = digest(
        &serde_json::to_vec(&mutation_rows)
            .map_err(|_| adapter_error(AdapterErrorId::ProjectionFailed))?,
    );
    let rollback_policy_sha256 = digest(
        &serde_json::to_vec(&(
            ROLLBACK_POLICY,
            &mutation_rows,
            request.plan.rollback.mutation_count,
        ))
        .map_err(|_| adapter_error(AdapterErrorId::ProjectionFailed))?,
    );
    Ok(PermitBinding {
        request_id: request.request_id.clone(),
        context_id: request.context_id.clone(),
        candidate_id: request.candidate_id.clone(),
        repository_root_id: request.target.repository_root_id.clone(),
        worktree_root_id: request.target.worktree_root_id.clone(),
        root_binding: request.root_binding.clone(),
        plan_record_sha256: digest(&request.plan_record_bytes),
        plan_sha256: request.plan.plan_sha256.clone(),
        accepted_plan_sha256: request.accepted_plan_sha256.clone(),
        desired_state_sha256: request.desired.state_sha256.clone(),
        source_manifest_sha256: request.authority.manifest_sha256.clone(),
        source_catalog_sha256: request.authority.catalog_sha256.clone(),
        source_authority_sha256: request.authority.authority_sha256.clone(),
        target_prestate_sha256: target.sha256.clone(),
        allowed_mutation_set_sha256,
        rollback_policy_sha256,
        protected_prestate_sha256: protected.sha256.clone(),
    })
}
