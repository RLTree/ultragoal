use super::*;

pub(crate) fn final_reconcile(
    current: Option<&super::super::Mutation>,
    mutations: &[super::super::Mutation],
    root_binding: &str,
    effects: &mut impl FitEffects,
) -> Result<(), FitError> {
    require_root(effects, root_binding).map_err(|_| error(FitErrorId::RollbackFailed))?;
    let mut failed = false;
    for mutation in current.into_iter().chain(mutations) {
        let observed = effects.read_file(
            &mutation.path,
            super::super::repository_contract::MAX_FILE_BYTES,
        );
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

pub(crate) fn restore_one(
    mutation: &super::super::Mutation,
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
        .read_file(
            &mutation.path,
            super::super::repository_contract::MAX_FILE_BYTES,
        )
        .map_err(|_| error(FitErrorId::RollbackFailed))?;
    if observed.as_deref() != mutation.prior.as_deref() {
        return Err(error(FitErrorId::RollbackFailed));
    }
    require_root(effects, root_binding).map_err(|_| error(FitErrorId::RollbackFailed))
}

pub(crate) fn require_root(reader: &mut impl FitReader, expected: &str) -> Result<(), FitError> {
    if reader.root_binding()? != expected {
        Err(error(FitErrorId::StaleBinding))
    } else {
        Ok(())
    }
}

pub(crate) fn require_preconditions(
    checks: &[super::super::FitCheck],
    reader: &mut impl FitReader,
) -> Result<(), FitError> {
    for check in checks {
        let observed = reader.read_file(
            &check.path,
            super::super::repository_contract::MAX_FILE_BYTES,
        )?;
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

pub(crate) fn require_postconditions(
    checks: &[super::super::FitCheck],
    reader: &mut impl FitReader,
) -> Result<(), FitError> {
    for check in checks {
        let observed = reader.read_file(
            &check.path,
            super::super::repository_contract::MAX_FILE_BYTES,
        )?;
        if observed
            .as_deref()
            .is_none_or(|bytes| digest(bytes) != check.desired_sha256)
        {
            return Err(error(FitErrorId::VerificationFailed));
        }
    }
    Ok(())
}
