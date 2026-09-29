pub(super) const MARKETPLACE_CATALOG_PATH: &str = ".agents/plugins/marketplace.json";

pub(crate) const ISOLATED_MARKETPLACE_NAME: &str = "harness-ultragoal-local";

pub(super) fn is_canonical_marketplace_catalog(bytes: &[u8]) -> bool {
    bytes == include_bytes!("../../../../../.agents/plugins/marketplace.json")
}

impl ProductionPackageArtifact {
    pub(crate) fn materialize_marketplace_catalog(
        &self,
        context: &LiveContext,
        catalog: &AuthorityCatalog,
        output: &ScopedFile,
    ) -> Result<(), ProductionPackageError> {
        verify_product_package(self, context, catalog)?;
        if output.relative_path() != MARKETPLACE_CATALOG_PATH {
            return Err(failure(ProductionPackageErrorId::OutputFailed));
        }
        let bytes = include_bytes!("../../../../../.agents/plugins/marketplace.json");
        match output.apply(None, Some(bytes)) {
            Ok(true) => {}
            Ok(false) | Err(_) => return Err(failure(ProductionPackageErrorId::OutputFailed)),
        }
        let valid = context.revalidate().is_ok()
            && verify_product_package(self, context, catalog).is_ok()
            && output.inspect(bytes.len()).ok().flatten().as_deref() == Some(bytes);
        if valid {
            Ok(())
        } else {
            let _ = output.apply(Some(&sha256(bytes)), None);
            Err(failure(ProductionPackageErrorId::OutputFailed))
        }
    }
}
