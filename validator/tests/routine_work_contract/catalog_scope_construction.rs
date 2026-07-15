use super::scope::{
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
