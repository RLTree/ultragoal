use super::*;
use crate::catalog_fixture_scope::{
    CatalogSetupFailurePoint, ClaimedFixtureScope, FixtureScopeError,
};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Mutex};

#[test]
pub(crate) fn catalog_fixture_drop_and_unwind_preserve_scope_until_explicit_teardown() {
    let root = TestRoot::new("drop-inert", VALID_CATALOG);
    let path = root.path.clone();
    let sentinel = path.join("drop-sentinel");
    fs::write(&sentinel, b"catalog fixture drop must not delete this\n").unwrap();
    drop(root);
    assert_eq!(
        fs::read(&sentinel).unwrap(),
        b"catalog fixture drop must not delete this\n"
    );
    fs::remove_dir_all(&path).expect("explicit catalog fixture teardown failed");
    assert!(!path.exists());

    let retained = Arc::new(Mutex::new(None::<PathBuf>));
    let captured = Arc::clone(&retained);
    let result = catch_unwind(AssertUnwindSafe(|| {
        let root = TestRoot::new("unwind-inert", VALID_CATALOG);
        fs::write(
            root.path.join("unwind-sentinel"),
            b"unwind keeps catalog fixture\n",
        )
        .unwrap();
        *captured.lock().unwrap() = Some(root.path.clone());
        panic!("induced catalog fixture unwind");
    }));
    assert!(result.is_err());
    let path = retained.lock().unwrap().take().unwrap();
    assert_eq!(
        fs::read(path.join("unwind-sentinel")).unwrap(),
        b"unwind keeps catalog fixture\n"
    );
    fs::remove_dir_all(&path).expect("explicit catalog unwind teardown failed");
    assert!(!path.exists());
}

#[test]
pub(crate) fn catalog_fixture_setup_failures_roll_back_only_the_claimed_child() {
    for (point, stage) in [
        (CatalogSetupFailurePoint::AfterClaim, "claim"),
        (CatalogSetupFailurePoint::AfterDirectories, "directories"),
        (CatalogSetupFailurePoint::AfterCatalogWrite, "catalog"),
    ] {
        let sequence = NEXT.load(Ordering::Relaxed);
        let error = match TestRoot::try_new("setup-rollback", VALID_CATALOG, Some(point)) {
            Ok(_) => panic!("setup failure {stage} unexpectedly constructed a fixture"),
            Err(error) => error,
        };
        assert_eq!(error, FixtureScopeError::Setup(stage));
        let path =
            fixture_parent().join(format!("setup-rollback-{}-{sequence}", std::process::id()));
        assert!(
            !path.exists(),
            "setup failure retained {stage}: {}",
            path.display()
        );
    }
}

#[test]
pub(crate) fn catalog_fixture_setup_rollback_refuses_a_substituted_scope() {
    let parent = fixture_parent();
    let name = format!(
        "setup-substitution-{}",
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    let mut scope = ClaimedFixtureScope::claim(&parent, &name).unwrap();
    let held = parent.join(format!("{name}-held"));
    fs::rename(scope.path(), &held).unwrap();
    fs::create_dir(scope.path()).unwrap();
    fs::write(scope.path().join("foreign"), b"preserve foreign scope\n").unwrap();
    assert_eq!(scope.rollback(), Err(FixtureScopeError::Substituted));
    assert_eq!(
        fs::read(scope.path().join("foreign")).unwrap(),
        b"preserve foreign scope\n"
    );
    fs::remove_dir_all(scope.path()).unwrap();
    fs::remove_dir_all(&held).unwrap();
}

fn fixture_parent() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("target/routine-production-catalog-fixtures")
}
