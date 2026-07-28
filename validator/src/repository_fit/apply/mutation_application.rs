use super::*;

pub fn apply(
    plan: &FitPlan,
    authorization: &PlanAuthorization,
    effects: &mut impl FitEffects,
) -> Result<AppliedFit, FitError> {
    if !authorization.matches(plan) {
        return Err(error(FitErrorId::Unauthorized));
    }
    if !plan.conflicts.is_empty() {
        return Err(error(FitErrorId::Conflict));
    }
    require_root(effects, &plan.root_binding)?;
    require_preconditions(&plan.checks, effects)?;
    require_root(effects, &plan.root_binding)?;
    let mutations = plan.all_mutations();
    let mut applied = 0usize;
    for mutation in &mutations {
        let result = effects.compare_exchange(
            &mutation.path,
            &mutation.expected,
            Some(&mutation.replacement),
        );
        match result {
            Ok(true) => applied += 1,
            Ok(false) => {
                return fail_with_rollback(
                    error(FitErrorId::Conflict),
                    &mutations[..applied],
                    &plan.root_binding,
                    effects,
                );
            }
            Err(failure) => {
                return reconcile_effect_error(
                    failure,
                    mutation,
                    &mutations[..applied],
                    &plan.root_binding,
                    effects,
                );
            }
        }
        if let Err(failure) = require_root(effects, &plan.root_binding) {
            return fail_with_rollback(failure, &mutations[..applied], &plan.root_binding, effects);
        }
    }
    if require_postconditions(&plan.checks, effects).is_err() {
        return fail_with_rollback(
            error(FitErrorId::VerificationFailed),
            &mutations[..applied],
            &plan.root_binding,
            effects,
        );
    }
    if let Err(failure) = require_root(effects, &plan.root_binding) {
        return fail_with_rollback(failure, &mutations[..applied], &plan.root_binding, effects);
    }
    Ok(AppliedFit {
        plan_sha256: plan.plan_sha256.clone(),
        root_binding: plan.root_binding.clone(),
        mutations,
    })
}

pub fn rollback(transaction: AppliedFit, effects: &mut impl FitEffects) -> Result<(), FitError> {
    require_root(effects, &transaction.root_binding)?;
    restore(&transaction.mutations, &transaction.root_binding, effects)
}

pub fn verify(
    desired: &DesiredState,
    reader: &mut impl FitReader,
) -> Result<FitVerification, FitError> {
    let root_binding = reader.root_binding()?;
    if !valid_digest(&root_binding) {
        return Err(error(FitErrorId::StaleBinding));
    }
    for file in &desired.files {
        match reader.read_file(
            &file.path,
            super::super::repository_contract::MAX_FILE_BYTES,
        )? {
            Some(bytes) if digest(&bytes) == file.sha256() => {}
            _ => return Err(error(FitErrorId::VerificationFailed)),
        }
    }
    if reader.root_binding()? != root_binding {
        return Err(error(FitErrorId::StaleBinding));
    }
    let encoded = serde_json::to_vec(&(
        &desired.context_id,
        &desired.candidate_id,
        &desired.state_sha256,
        &root_binding,
        desired.files.len(),
        true,
    ))
    .map_err(|_| error(FitErrorId::InvalidSpec))?;
    Ok(FitVerification {
        context_id: desired.context_id.clone(),
        candidate_id: desired.candidate_id.clone(),
        desired_state_sha256: desired.state_sha256.clone(),
        root_binding,
        matched_files: desired.files.len(),
        idempotent: true,
        verification_sha256: digest(&encoded),
    })
}

pub(crate) fn fail_with_rollback<T>(
    failure: FitError,
    applied: &[super::super::Mutation],
    root_binding: &str,
    effects: &mut impl FitEffects,
) -> Result<T, FitError> {
    if restore(applied, root_binding, effects).is_err() {
        Err(error(FitErrorId::RollbackFailed))
    } else {
        Err(failure)
    }
}

pub(crate) fn reconcile_effect_error<T>(
    failure: FitError,
    current: &super::super::Mutation,
    applied: &[super::super::Mutation],
    root_binding: &str,
    effects: &mut impl FitEffects,
) -> Result<T, FitError> {
    let mut failed = match observe_after_error(current, root_binding, effects) {
        Ok(observed) if observed.as_deref() == current.prior.as_deref() => false,
        Ok(observed) if observed.as_deref() == Some(current.replacement.as_slice()) => {
            restore_one(current, root_binding, effects).is_err()
        }
        _ => true,
    };
    failed |= attempt_restore(applied, root_binding, effects).is_err();
    failed |= final_reconcile(Some(current), applied, root_binding, effects).is_err();
    if failed {
        Err(error(FitErrorId::RollbackFailed))
    } else {
        Err(failure)
    }
}

pub(crate) fn observe_after_error(
    mutation: &super::super::Mutation,
    root_binding: &str,
    effects: &mut impl FitEffects,
) -> Result<Option<Vec<u8>>, FitError> {
    require_root(effects, root_binding)?;
    let observed = effects.read_file(
        &mutation.path,
        super::super::repository_contract::MAX_FILE_BYTES,
    )?;
    require_root(effects, root_binding)?;
    Ok(observed)
}

pub(crate) fn restore(
    mutations: &[super::super::Mutation],
    root_binding: &str,
    effects: &mut impl FitEffects,
) -> Result<(), FitError> {
    let restore_failed = attempt_restore(mutations, root_binding, effects).is_err();
    let reconcile_failed = final_reconcile(None, mutations, root_binding, effects).is_err();
    if restore_failed || reconcile_failed {
        Err(error(FitErrorId::RollbackFailed))
    } else {
        Ok(())
    }
}

pub(crate) fn attempt_restore(
    mutations: &[super::super::Mutation],
    root_binding: &str,
    effects: &mut impl FitEffects,
) -> Result<(), FitError> {
    require_root(effects, root_binding).map_err(|_| error(FitErrorId::RollbackFailed))?;
    let mut failed = false;
    for mutation in mutations.iter().rev() {
        failed |= restore_one(mutation, root_binding, effects).is_err();
    }
    failed |= require_root(effects, root_binding).is_err();
    if failed {
        Err(error(FitErrorId::RollbackFailed))
    } else {
        Ok(())
    }
}
