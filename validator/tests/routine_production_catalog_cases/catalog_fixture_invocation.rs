use super::*;
use crate::catalog_fixture_claim::{ClaimResidue, FixtureClaimFailure};
use crate::catalog_fixture_scope::{ClaimedFixtureScope, FixtureScopeError};
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

pub(crate) struct CatalogFixtureInvocation {
    scope: ClaimedFixtureScope,
}

pub(crate) fn run_catalog_case(label: &str, body: impl FnOnce(&mut CatalogFixtureInvocation)) {
    let mut invocation = CatalogFixtureInvocation::begin(label, None);
    let result = catch_unwind(AssertUnwindSafe(|| body(&mut invocation)));
    finish_invocation(invocation);
    if let Err(payload) = result {
        resume_unwind(payload);
    }
}

pub(crate) fn run_catalog_case_with_begin_failure(
    label: &str,
    failure: crate::catalog_fixture_claim::ClaimFailurePoint,
) {
    let invocation = CatalogFixtureInvocation::begin(label, Some(failure));
    finish_invocation(invocation);
}

impl CatalogFixtureInvocation {
    fn begin(
        label: &str,
        failure: Option<crate::catalog_fixture_claim::ClaimFailurePoint>,
    ) -> Self {
        let name = format!(
            "invocation-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let scope =
            match ClaimedFixtureScope::claim_with_failure(&invocation_parent(), &name, failure) {
                Ok(scope) => scope,
                Err(FixtureClaimFailure::Failed(error)) => {
                    panic!("catalog invocation claim failed: {error:?}")
                }
                Err(FixtureClaimFailure::Retained(residue)) => {
                    settle_claim_residue(residue);
                    panic!("catalog invocation claim retained before setup")
                }
            };
        Self { scope }
    }

    pub(crate) fn new_root(&mut self, label: &str, catalog_bytes: &[u8]) -> TestRoot {
        TestRoot::new_in(&self.scope, label, catalog_bytes, None, None).unwrap_or_else(|error| {
            panic!("catalog fixture setup failed after settlement: {error:?}")
        })
    }

    pub(crate) fn try_root(
        &mut self,
        label: &str,
        catalog_bytes: &[u8],
        fail_after: Option<crate::catalog_fixture_scope::CatalogSetupFailurePoint>,
        claim_failure: Option<crate::catalog_fixture_claim::ClaimFailurePoint>,
    ) -> Result<TestRoot, FixtureScopeError> {
        TestRoot::new_in(&self.scope, label, catalog_bytes, fail_after, claim_failure)
    }

    fn finish(mut self) -> Result<(), Self> {
        match self.scope.teardown_after_assertions() {
            Ok(()) => Ok(()),
            Err(_) => Err(self),
        }
    }
}

fn invocation_parent() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("catalog fixture manifest has no workspace parent")
        .join("target/routine-production-catalog-fixtures")
}

fn finish_invocation(invocation: CatalogFixtureInvocation) {
    match invocation.finish() {
        Ok(()) => {}
        Err(invocation) => match invocation.finish() {
            Ok(()) => {}
            Err(_) => panic!("catalog invocation cleanup remained in typed retained custody"),
        },
    }
}

fn settle_claim_residue(residue: ClaimResidue) {
    let mut scope = match residue.reconcile() {
        Ok(scope) => scope,
        Err(_) => panic!("catalog invocation retained claim could not be reconciled"),
    };
    match scope.rollback() {
        Ok(()) => {}
        Err(_) => match scope.rollback() {
            Ok(()) => {}
            Err(_) => panic!("catalog invocation retained claim could not be settled"),
        },
    }
}
