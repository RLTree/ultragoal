use super::*;
use crate::catalog_fixture_claim::{ClaimFailurePoint, ClaimResidue, FixtureClaimFailure};
use crate::catalog_fixture_scope::{
    CatalogSetupFailurePoint, ClaimedFixtureScope, FixtureScopeError, populate_catalog_scope,
};

pub(crate) enum FixtureConstructionFailure {
    Setup(FixtureScopeError),
    Claim(ClaimResidue),
    Scope {
        scope: ClaimedFixtureScope,
        error: FixtureScopeError,
    },
}

impl TestRoot {
    pub(crate) fn new_in(
        parent: &ClaimedFixtureScope,
        label: &str,
        catalog_bytes: &[u8],
        fail_after: Option<CatalogSetupFailurePoint>,
        claim_failure: Option<ClaimFailurePoint>,
    ) -> Result<Self, FixtureConstructionFailure> {
        let name = format!("{label}-{}", NEXT.fetch_add(1, Ordering::Relaxed));
        let mut scope = match ClaimedFixtureScope::claim_child(parent, &name, claim_failure) {
            Ok(scope) => scope,
            Err(FixtureClaimFailure::Failed(error)) => {
                return Err(FixtureConstructionFailure::Setup(error));
            }
            Err(FixtureClaimFailure::Retained(residue)) => {
                return Err(FixtureConstructionFailure::Claim(residue));
            }
        };
        if let Err(error) = populate_catalog_scope(scope.path(), catalog_bytes, fail_after) {
            return match scope.rollback() {
                Ok(()) => Err(FixtureConstructionFailure::Setup(error)),
                Err(cleanup) => Err(FixtureConstructionFailure::Scope {
                    scope,
                    error: cleanup,
                }),
            };
        }
        Ok(Self {
            path: scope.path().to_path_buf(),
            scope,
        })
    }
}
