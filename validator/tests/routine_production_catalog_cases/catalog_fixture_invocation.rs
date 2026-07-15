use super::*;
use crate::catalog_fixture::FixtureRootGuard;
use crate::catalog_fixture_claim::FixtureClaimFailure;
use crate::catalog_fixture_construction::FixtureConstructionFailure;
use crate::catalog_fixture_scope::ClaimedFixtureScope;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};

pub(crate) struct CatalogFixtureInvocation {
    scope: ClaimedFixtureScope,
}

pub(crate) fn run_catalog_case(
    label: &str,
    body: impl FnOnce(&mut CatalogFixtureInvocation) -> Result<(), FixtureConstructionFailure>,
) {
    let guard = crate::catalog_fixture::lock_fixture_root();
    run_catalog_case_with_guard(guard, label, body);
}

pub(crate) fn run_catalog_case_with_guard(
    guard: FixtureRootGuard,
    label: &str,
    body: impl FnOnce(&mut CatalogFixtureInvocation) -> Result<(), FixtureConstructionFailure>,
) {
    let mut invocation = begin_until_settled(label, None);
    let result = catch_unwind(AssertUnwindSafe(|| body(&mut invocation)));
    let body_failure = result.as_ref().ok().and_then(|value| value.as_ref().err());
    finish_until_settled(invocation);
    drop(guard);
    if let Some(failure) = body_failure {
        panic!(
            "catalog fixture body failed after outer settlement: {:?}",
            failure_code(failure)
        );
    }
    if let Err(payload) = result {
        resume_unwind(payload);
    }
}

pub(crate) fn run_catalog_case_with_begin_failure_guard(
    guard: FixtureRootGuard,
    label: &str,
    failure: crate::catalog_fixture_claim::ClaimFailurePoint,
) {
    let invocation = begin_until_settled(label, Some(failure));
    finish_until_settled(invocation);
    drop(guard);
    panic!("catalog invocation claim unexpectedly succeeded");
}

impl CatalogFixtureInvocation {
    pub(crate) fn new_root(
        &mut self,
        label: &str,
        catalog_bytes: &[u8],
    ) -> Result<TestRoot, FixtureConstructionFailure> {
        TestRoot::new_in(&self.scope, label, catalog_bytes, None, None)
    }

    pub(crate) fn try_root(
        &mut self,
        label: &str,
        catalog_bytes: &[u8],
        fail_after: Option<crate::catalog_fixture_scope::CatalogSetupFailurePoint>,
        claim_failure: Option<crate::catalog_fixture_claim::ClaimFailurePoint>,
    ) -> Result<TestRoot, FixtureConstructionFailure> {
        TestRoot::new_in(&self.scope, label, catalog_bytes, fail_after, claim_failure)
    }

    fn finish(mut self) -> Result<(), Self> {
        match self.scope.teardown_after_assertions() {
            Ok(()) => Ok(()),
            Err(_) => Err(self),
        }
    }
}

fn begin_until_settled(
    label: &str,
    failure: Option<crate::catalog_fixture_claim::ClaimFailurePoint>,
) -> CatalogFixtureInvocation {
    let name = format!(
        "invocation-{label}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    match ClaimedFixtureScope::claim_with_failure(&invocation_parent(), &name, failure) {
        Ok(scope) => CatalogFixtureInvocation { scope },
        Err(FixtureClaimFailure::Failed(error)) => {
            panic!("catalog invocation claim failed: {error:?}")
        }
        Err(FixtureClaimFailure::Retained(residue)) => {
            settle_failure_until_done(FixtureConstructionFailure::Claim(residue));
            panic!("catalog invocation claim failed after settlement")
        }
    }
}

fn finish_until_settled(invocation: CatalogFixtureInvocation) {
    let mut pending = invocation;
    loop {
        pending = match pending.finish() {
            Ok(()) => return,
            Err(owner) => owner,
        };
    }
}

fn settle_failure_until_done(mut failure: FixtureConstructionFailure) {
    loop {
        failure = match settle_failure(failure) {
            Ok(()) => return,
            Err(retained) => retained,
        };
    }
}

fn settle_failure(failure: FixtureConstructionFailure) -> Result<(), FixtureConstructionFailure> {
    match failure {
        FixtureConstructionFailure::Setup(_) => Ok(()),
        FixtureConstructionFailure::Claim(residue) => match residue.reconcile() {
            Ok(mut scope) => match scope.rollback() {
                Ok(()) => Ok(()),
                Err(error) => Err(FixtureConstructionFailure::Scope { scope, error }),
            },
            Err(residue) => Err(FixtureConstructionFailure::Claim(residue)),
        },
        FixtureConstructionFailure::Scope { mut scope, error } => match scope.rollback() {
            Ok(()) => Ok(()),
            Err(_) => Err(FixtureConstructionFailure::Scope { scope, error }),
        },
    }
}

fn failure_code(failure: &FixtureConstructionFailure) -> &'static str {
    match failure {
        FixtureConstructionFailure::Setup(_) => "setup",
        FixtureConstructionFailure::Claim(_) => "claim",
        FixtureConstructionFailure::Scope { .. } => "scope",
    }
}

fn invocation_parent() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("catalog fixture manifest has no workspace parent")
        .join("target/routine-production-catalog-fixtures")
}
