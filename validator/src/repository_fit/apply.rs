use super::{
    AppliedFit, DesiredState, ExpectedContent, FitEffects, FitError, FitErrorId, FitPlan,
    FitReader, FitVerification, PlanAuthorization, digest, error, valid_digest,
};

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
    let mut applied = 0usize;
    for mutation in &plan.mutations {
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
                    &plan.mutations[..applied],
                    &plan.root_binding,
                    effects,
                );
            }
            Err(failure) => {
                return reconcile_effect_error(
                    failure,
                    mutation,
                    &plan.mutations[..applied],
                    &plan.root_binding,
                    effects,
                );
            }
        }
        if let Err(failure) = require_root(effects, &plan.root_binding) {
            return fail_with_rollback(
                failure,
                &plan.mutations[..applied],
                &plan.root_binding,
                effects,
            );
        }
    }
    if require_postconditions(&plan.checks, effects).is_err() {
        return fail_with_rollback(
            error(FitErrorId::VerificationFailed),
            &plan.mutations[..applied],
            &plan.root_binding,
            effects,
        );
    }
    if let Err(failure) = require_root(effects, &plan.root_binding) {
        return fail_with_rollback(
            failure,
            &plan.mutations[..applied],
            &plan.root_binding,
            effects,
        );
    }
    Ok(AppliedFit {
        plan_sha256: plan.plan_sha256.clone(),
        root_binding: plan.root_binding.clone(),
        mutations: plan.mutations.clone(),
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
        match reader.read_file(&file.path, super::model::MAX_FILE_BYTES)? {
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

fn fail_with_rollback<T>(
    failure: FitError,
    applied: &[super::Mutation],
    root_binding: &str,
    effects: &mut impl FitEffects,
) -> Result<T, FitError> {
    if restore(applied, root_binding, effects).is_err() {
        Err(error(FitErrorId::RollbackFailed))
    } else {
        Err(failure)
    }
}

fn reconcile_effect_error<T>(
    failure: FitError,
    current: &super::Mutation,
    applied: &[super::Mutation],
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

fn observe_after_error(
    mutation: &super::Mutation,
    root_binding: &str,
    effects: &mut impl FitEffects,
) -> Result<Option<Vec<u8>>, FitError> {
    require_root(effects, root_binding)?;
    let observed = effects.read_file(&mutation.path, super::model::MAX_FILE_BYTES)?;
    require_root(effects, root_binding)?;
    Ok(observed)
}

fn restore(
    mutations: &[super::Mutation],
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

fn attempt_restore(
    mutations: &[super::Mutation],
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

fn final_reconcile(
    current: Option<&super::Mutation>,
    mutations: &[super::Mutation],
    root_binding: &str,
    effects: &mut impl FitEffects,
) -> Result<(), FitError> {
    require_root(effects, root_binding).map_err(|_| error(FitErrorId::RollbackFailed))?;
    let mut failed = false;
    for mutation in current.into_iter().chain(mutations) {
        let observed = effects.read_file(&mutation.path, super::model::MAX_FILE_BYTES);
        failed |=
            !matches!(observed, Ok(observed) if observed.as_deref() == mutation.prior.as_deref());
    }
    failed |= require_root(effects, root_binding).is_err();
    if failed {
        Err(error(FitErrorId::RollbackFailed))
    } else {
        Ok(())
    }
}

fn restore_one(
    mutation: &super::Mutation,
    root_binding: &str,
    effects: &mut impl FitEffects,
) -> Result<(), FitError> {
    require_root(effects, root_binding).map_err(|_| error(FitErrorId::RollbackFailed))?;
    let expected = ExpectedContent::ExactDigest(mutation.replacement_sha256());
    match effects.compare_exchange(&mutation.path, &expected, mutation.prior.as_deref()) {
        Ok(true) => {}
        _ => return Err(error(FitErrorId::RollbackFailed)),
    }
    let observed = effects
        .read_file(&mutation.path, super::model::MAX_FILE_BYTES)
        .map_err(|_| error(FitErrorId::RollbackFailed))?;
    if observed.as_deref() != mutation.prior.as_deref() {
        return Err(error(FitErrorId::RollbackFailed));
    }
    require_root(effects, root_binding).map_err(|_| error(FitErrorId::RollbackFailed))
}

fn require_root(reader: &mut impl FitReader, expected: &str) -> Result<(), FitError> {
    if reader.root_binding()? != expected {
        Err(error(FitErrorId::StaleBinding))
    } else {
        Ok(())
    }
}

fn require_preconditions(
    checks: &[super::FitCheck],
    reader: &mut impl FitReader,
) -> Result<(), FitError> {
    for check in checks {
        let observed = reader.read_file(&check.path, super::model::MAX_FILE_BYTES)?;
        let matches = match &check.expected {
            ExpectedContent::Absent => observed.is_none(),
            ExpectedContent::ExactDigest(expected) => observed
                .as_deref()
                .is_some_and(|bytes| digest(bytes) == *expected),
        };
        if !matches {
            return Err(error(FitErrorId::Conflict));
        }
    }
    Ok(())
}

fn require_postconditions(
    checks: &[super::FitCheck],
    reader: &mut impl FitReader,
) -> Result<(), FitError> {
    for check in checks {
        let observed = reader.read_file(&check.path, super::model::MAX_FILE_BYTES)?;
        if !observed
            .as_deref()
            .is_some_and(|bytes| digest(bytes) == check.desired_sha256)
        {
            return Err(error(FitErrorId::VerificationFailed));
        }
    }
    Ok(())
}
