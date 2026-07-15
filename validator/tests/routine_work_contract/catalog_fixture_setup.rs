use super::catalog_fixture_scope::{
    CatalogSetupFailurePoint, ClaimedFixtureScope, FixtureScopeError, populate_catalog_scope,
};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

#[test]
fn claimed_catalog_scope_rolls_back_setup_failures_and_refuses_substitution() {
    let parent = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("target/routine-work-contract-catalog-fixture-controls");
    for (point, stage) in [
        (CatalogSetupFailurePoint::AfterClaim, "claim"),
        (CatalogSetupFailurePoint::AfterDirectories, "directories"),
        (CatalogSetupFailurePoint::AfterCatalogWrite, "catalog"),
    ] {
        let name = format!(
            "claim-{stage}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
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
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    let mut scope = ClaimedFixtureScope::claim(&parent, &name).unwrap();
    let held = parent.join(format!("{name}-held"));
    fs::rename(scope.path(), &held).unwrap();
    fs::create_dir(scope.path()).unwrap();
    fs::write(scope.path().join("foreign"), b"do not delete\n").unwrap();
    assert_eq!(scope.rollback(), Err(FixtureScopeError::Substituted));
    assert_eq!(
        fs::read(scope.path().join("foreign")).unwrap(),
        b"do not delete\n"
    );
    fs::remove_dir_all(scope.path()).unwrap();
    fs::remove_dir_all(held).unwrap();
}
