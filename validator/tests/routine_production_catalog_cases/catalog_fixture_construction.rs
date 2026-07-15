use super::*;
use crate::catalog_fixture_scope::{
    CatalogSetupFailurePoint, ClaimedFixtureScope, FixtureScopeError, populate_catalog_scope,
};

impl TestRoot {
    pub(crate) fn new(label: &str, catalog_bytes: &[u8]) -> Self {
        Self::try_new(label, catalog_bytes, None)
            .unwrap_or_else(|error| panic!("catalog fixture setup failed: {error:?}"))
    }

    pub(crate) fn try_new(
        label: &str,
        catalog_bytes: &[u8],
        fail_after: Option<CatalogSetupFailurePoint>,
    ) -> Result<Self, FixtureScopeError> {
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace = manifest.parent().ok_or_else(|| {
            FixtureScopeError::Claim("manifest has no workspace parent".to_owned())
        })?;
        let parent = workspace.join("target/routine-production-catalog-fixtures");
        let name = format!("{label}-{}-{sequence}", std::process::id());
        let mut scope = ClaimedFixtureScope::claim(&parent, &name)?;
        if let Err(error) = populate_catalog_scope(scope.path(), catalog_bytes, fail_after) {
            scope.rollback()?;
            return Err(error);
        }
        Ok(Self {
            path: scope.release(),
        })
    }
}
