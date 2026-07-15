use super::*;
use crate::catalog_fixture_claim::{ClaimFailurePoint, set_reconciliation_refusals};
use crate::catalog_fixture_cleanup_hook::{set_before_final_removal, set_final_refusals};
use crate::catalog_fixture_scope::CatalogSetupFailurePoint;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[test]
pub(crate) fn invocation_begin_failure_settles_before_the_test_harness_boundary() {
    let before = fixture_inventory();
    set_reconciliation_refusals(2);
    let result = catch_unwind(AssertUnwindSafe(|| {
        crate::catalog_fixture_invocation::run_catalog_case_with_begin_failure(
            "begin-failure",
            ClaimFailurePoint::AfterMkdirBeforeOpen,
        );
    }));
    assert!(result.is_err());
    assert_eq!(fixture_inventory(), before);
}

#[test]
pub(crate) fn invocation_finish_retries_a_temporary_refusal_while_custody_is_live() {
    let before = fixture_inventory();
    set_before_final_removal(None);
    set_final_refusals(2);
    crate::catalog_fixture_invocation::run_catalog_case("finish-refusal", |invocation| {
        let root = invocation.new_root("finish-refusal-root", VALID_CATALOG)?;
        let path = root.path().to_path_buf();
        drop(root);
        assert!(
            path.exists(),
            "root drop must not consume invocation custody"
        );
        Ok(())
    });
    assert_eq!(fixture_inventory(), before);
}

#[test]
pub(crate) fn invocation_settles_a_body_construction_failure_before_returning_it_as_non_custody() {
    let before = fixture_inventory();
    let result = catch_unwind(AssertUnwindSafe(|| {
        crate::catalog_fixture_invocation::run_catalog_case("body-failure", |invocation| {
            let _ = invocation.try_root(
                "body-failure-root",
                VALID_CATALOG,
                Some(CatalogSetupFailurePoint::AfterDirectories),
                None,
            )?;
            Ok(())
        });
    }));
    assert!(result.is_err());
    assert_eq!(fixture_inventory(), before);
}

#[test]
pub(crate) fn invocation_settles_after_an_uncaught_body_unwind() {
    let before = fixture_inventory();
    let result = catch_unwind(AssertUnwindSafe(|| {
        crate::catalog_fixture_invocation::run_catalog_case("body-unwind", |invocation| {
            let _root = invocation.new_root("body-unwind-root", VALID_CATALOG)?;
            panic!("induced outer invocation unwind");
        });
    }));
    assert!(result.is_err());
    assert_eq!(fixture_inventory(), before);
}

fn fixture_inventory() -> Vec<PathBuf> {
    let parent = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("target/routine-production-catalog-fixtures");
    let mut paths = fs::read_dir(parent)
        .map(|rows| {
            rows.filter_map(Result::ok)
                .map(|row| row.path())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    paths.sort();
    paths
}
