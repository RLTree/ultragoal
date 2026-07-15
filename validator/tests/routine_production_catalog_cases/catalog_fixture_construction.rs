use super::*;
use crate::catalog_fixture_scope::{
    CatalogSetupFailurePoint, ClaimedFixtureScope, FixtureScopeError, populate_catalog_scope,
};

pub(crate) enum FixtureConstructionFailure {
    Setup(FixtureScopeError),
    Retained {
        scope: ClaimedFixtureScope,
        error: FixtureScopeError,
    },
}

impl std::fmt::Debug for FixtureConstructionFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Setup(error) => formatter.debug_tuple("Setup").field(error).finish(),
            Self::Retained { scope, error } => formatter
                .debug_struct("Retained")
                .field("path", &scope.path())
                .field("error", error)
                .finish(),
        }
    }
}

impl TestRoot {
    pub(crate) fn new(label: &str, catalog_bytes: &[u8]) -> Self {
        Self::try_new(label, catalog_bytes, None)
            .unwrap_or_else(|error| panic!("catalog fixture setup failed: {error:?}"))
    }

    pub(crate) fn try_new(
        label: &str,
        catalog_bytes: &[u8],
        fail_after: Option<CatalogSetupFailurePoint>,
    ) -> Result<Self, FixtureConstructionFailure> {
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace = manifest
            .parent()
            .ok_or_else(|| FixtureScopeError::Claim("manifest has no workspace parent".to_owned()))
            .map_err(FixtureConstructionFailure::Setup)?;
        let parent = workspace.join("target/routine-production-catalog-fixtures");
        let name = format!("{label}-{}-{sequence}", std::process::id());
        let mut scope = ClaimedFixtureScope::claim(&parent, &name)
            .map_err(FixtureConstructionFailure::Setup)?;
        if let Err(error) = populate_catalog_scope(scope.path(), catalog_bytes, fail_after) {
            return match scope.rollback() {
                Ok(()) => Err(FixtureConstructionFailure::Setup(error)),
                Err(cleanup) => Err(FixtureConstructionFailure::Retained {
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
