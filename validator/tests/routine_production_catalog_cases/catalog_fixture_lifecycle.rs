use super::*;
use crate::catalog_fixture_claim::{ClaimFailurePoint, ClaimResidue, FixtureClaimFailure};
use crate::catalog_fixture_construction::FixtureConstructionFailure;
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
            Err(FixtureConstructionFailure::Setup(error)) => error,
            Err(FixtureConstructionFailure::Retained { scope, error }) => panic!(
                "setup failure {stage} retained {}: {error:?}",
                scope.path().display()
            ),
            Err(FixtureConstructionFailure::ClaimRetained(residue)) => panic!(
                "setup failure {stage} retained {}: {:?}",
                residue.path().display(),
                residue.error()
            ),
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
    let parent = fixture_parent().join(format!(
        "pre-rollback-parent-{}",
        NEXT.load(Ordering::Relaxed)
    ));
    fs::create_dir_all(&parent).unwrap();
    let name = format!(
        "setup-substitution-{}",
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    let mut scope = ClaimedFixtureScope::claim(&parent, &name).unwrap();
    let held = parent.join(format!("{name}-held"));
    fs::rename(scope.path(), &held).unwrap();
    fs::create_dir(scope.path()).unwrap();
    fs::write(scope.path().join("foreign"), b"preserve foreign scope\n").unwrap();
    assert!(matches!(
        scope.rollback(),
        Err(FixtureScopeError::Retained(_))
    ));
    assert!(
        !scope.has_name_binding(),
        "the descriptor-held scope must not claim the replacement pathname"
    );
    assert_foreign_present(&parent);
    fs::remove_dir_all(&parent).unwrap();
}

#[test]
pub(crate) fn catalog_claim_failures_retain_typed_custody_without_uncertain_cleanup() {
    let base = fixture_parent();
    let name = format!(
        "claim-failure-parent-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    let mut parent_scope = ClaimedFixtureScope::claim(&base, &name).unwrap();
    let parent = parent_scope.path().to_path_buf();
    let unopened = ClaimedFixtureScope::claim_with_failure(
        &parent,
        "before-open",
        Some(ClaimFailurePoint::AfterMkdirBeforeOpen),
    );
    let residue = match unopened {
        Err(FixtureClaimFailure::Retained(ClaimResidue::Unopened(value))) => {
            ClaimResidue::Unopened(value)
        }
        _ => panic!("expected typed pre-open residue"),
    };
    let mut scope = residue.reconcile().unwrap();
    scope.rollback().unwrap();
    assert!(!parent.join("before-open").exists());
    assert!(matches!(
        scope.rollback(),
        Err(FixtureScopeError::Retained(_))
    ));

    let foreign = ClaimedFixtureScope::claim_with_failure(
        &parent,
        "foreign-retry",
        Some(ClaimFailurePoint::AfterMkdirBeforeOpen),
    );
    let residue = match foreign {
        Err(FixtureClaimFailure::Retained(ClaimResidue::Unopened(value))) => {
            ClaimResidue::Unopened(value)
        }
        _ => panic!("expected foreign-retry residue"),
    };
    fs::rename(parent.join("foreign-retry"), parent.join("foreign-held")).unwrap();
    fs::create_dir(parent.join("foreign-retry")).unwrap();
    fs::write(parent.join("foreign-retry/foreign"), b"preserve foreign\n").unwrap();
    assert!(matches!(residue.reconcile(), Err(ClaimResidue::Opened(_))));
    assert_eq!(
        fs::read(parent.join("foreign-retry/foreign")).unwrap(),
        b"preserve foreign\n"
    );
    parent_scope.teardown_after_assertions().unwrap();
}

#[test]
pub(crate) fn opened_claim_reconciliation_refuses_a_replaced_name() {
    let parent = fixture_parent().join(format!(
        "claim-replacement-parent-{}",
        NEXT.load(Ordering::Relaxed)
    ));
    fs::create_dir_all(&parent).unwrap();
    let name = "replacement";
    let residue = match ClaimedFixtureScope::claim_with_failure(
        &parent,
        name,
        Some(ClaimFailurePoint::AfterOpenBeforeIdentity),
    ) {
        Err(FixtureClaimFailure::Retained(residue)) => residue,
        _ => panic!("expected opened residue"),
    };
    let held = parent.join("held");
    fs::rename(parent.join(name), &held).unwrap();
    fs::create_dir(parent.join(name)).unwrap();
    fs::write(parent.join(name).join("foreign"), b"preserve foreign\n").unwrap();
    assert!(matches!(residue.reconcile(), Err(ClaimResidue::Opened(_))));
    assert_eq!(
        fs::read(parent.join(name).join("foreign")).unwrap(),
        b"preserve foreign\n"
    );
    fs::remove_dir_all(&parent).unwrap();
}

fn assert_foreign_present(parent: &Path) {
    let found = fs::read_dir(parent)
        .unwrap()
        .filter_map(Result::ok)
        .any(|entry| entry.path().join("foreign").is_file());
    assert!(found, "substituted foreign scope was removed");
}

fn fixture_parent() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("target/routine-production-catalog-fixtures")
}
