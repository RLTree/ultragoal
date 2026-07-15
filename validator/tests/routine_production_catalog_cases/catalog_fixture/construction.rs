use super::claim::{ClaimFailurePoint, ClaimResidue, FixtureClaimFailure};
use super::scope::{
    CatalogSetupFailurePoint, ClaimedFixtureScope, FixtureScopeError, populate_catalog_scope,
};
use super::*;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static CONSTRUCTION_CONTROL_SEQUENCE: AtomicU64 = AtomicU64::new(0);

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

pub(super) fn verify_catalog_scope_construction() {
    let parent = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("target/routine-work-contract-catalog-fixture-controls")
        .join(format!("scope-controls-{}", std::process::id()));
    fs::create_dir_all(&parent).unwrap();
    for (point, stage) in [
        (CatalogSetupFailurePoint::AfterClaim, "claim"),
        (CatalogSetupFailurePoint::AfterDirectories, "directories"),
        (CatalogSetupFailurePoint::AfterCatalogWrite, "catalog"),
    ] {
        let name = format!(
            "claim-{stage}-{}-{}",
            std::process::id(),
            CONSTRUCTION_CONTROL_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        );
        let mut scope = ClaimedFixtureScope::claim(&parent, &name).unwrap();
        assert_eq!(
            populate_catalog_scope(scope.path(), b"{}", Some(point)),
            Err(FixtureScopeError::Setup(stage))
        );
        scope.rollback().unwrap();
        assert!(!parent.join(&name).exists());
    }

    let name = format!(
        "substitution-{}-{}",
        std::process::id(),
        CONSTRUCTION_CONTROL_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    );
    let mut scope = ClaimedFixtureScope::claim(&parent, &name).unwrap();
    fs::rename(scope.path(), parent.join(format!("{name}-held"))).unwrap();
    fs::create_dir(scope.path()).unwrap();
    fs::write(scope.path().join("foreign"), b"do not delete\n").unwrap();
    assert!(matches!(
        scope.rollback(),
        Err(FixtureScopeError::Retained(_))
    ));
    assert!(!scope.has_name_binding());
    let foreign = fs::read_dir(&parent)
        .unwrap()
        .filter_map(Result::ok)
        .any(|entry| entry.path().join("foreign").is_file());
    assert!(foreign, "substituted fixture scope was removed");
    fs::remove_dir_all(parent).unwrap();
}

#[test]
fn catalog_scope_construction_rolls_back_owned_setup() {
    super::verify_catalog_scope_construction();
}
