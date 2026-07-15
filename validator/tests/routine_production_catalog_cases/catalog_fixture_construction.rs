use super::*;
use crate::catalog_fixture_claim::{ClaimFailurePoint, ClaimResidue, FixtureClaimFailure};
use crate::catalog_fixture_scope::{
    CatalogSetupFailurePoint, ClaimedFixtureScope, FixtureScopeError, populate_catalog_scope,
};

impl TestRoot {
    pub(crate) fn new_in(
        parent: &ClaimedFixtureScope,
        label: &str,
        catalog_bytes: &[u8],
        fail_after: Option<CatalogSetupFailurePoint>,
        claim_failure: Option<ClaimFailurePoint>,
    ) -> Result<Self, FixtureScopeError> {
        let name = format!("{label}-{}", NEXT.fetch_add(1, Ordering::Relaxed));
        let mut scope = match ClaimedFixtureScope::claim_child(parent, &name, claim_failure) {
            Ok(scope) => scope,
            Err(FixtureClaimFailure::Failed(error)) => return Err(error),
            Err(FixtureClaimFailure::Retained(residue)) => return settle_claim(residue),
        };
        if let Err(error) = populate_catalog_scope(scope.path(), catalog_bytes, fail_after) {
            return match scope.rollback() {
                Ok(()) => Err(error),
                Err(cleanup) => Err(cleanup),
            };
        }
        Ok(Self {
            path: scope.path().to_path_buf(),
            scope,
        })
    }
}

fn settle_claim(residue: ClaimResidue) -> Result<TestRoot, FixtureScopeError> {
    let mut scope = residue.reconcile().map_err(|value| {
        FixtureScopeError::Retained(format!("claim residue retained: {:?}", value.error()))
    })?;
    let error = scope
        .rollback()
        .err()
        .unwrap_or_else(|| FixtureScopeError::Retained("claim setup rolled back".to_owned()));
    Err(error)
}
