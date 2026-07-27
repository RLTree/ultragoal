const ARCHIVE_OUTPUT_LIMIT: usize = 65 * 1024 * 1024;

impl ProductionPackageArtifact {
    pub(crate) fn publish_archive(
        &self,
        context: &LiveContext,
        catalog: &AuthorityCatalog,
        output: &ScopedFile,
    ) -> Result<(), ProductionPackageError> {
        verify_product_package(self, context, catalog)?;
        let previous = output
            .inspect(ARCHIVE_OUTPUT_LIMIT)
            .map_err(|_| failure(ProductionPackageErrorId::OutputFailed))?;
        let replacement = self.snapshot.archive();
        let expected = previous.as_deref().map(sha256);
        match output.apply(expected.as_deref(), Some(replacement)) {
            Ok(true) => {}
            Ok(false) | Err(_) => return Err(failure(ProductionPackageErrorId::OutputFailed)),
        }
        let post_effect = context
            .revalidate()
            .map_err(|_| failure(ProductionPackageErrorId::SourceUnavailable))
            .and_then(|_| verify_product_package(self, context, catalog))
            .and_then(|_| verify_archive_output(output, replacement));
        if let Err(problem) = post_effect {
            restore_archive_output(output, replacement, previous.as_deref())?;
            return Err(problem);
        }
        Ok(())
    }
}

fn verify_archive_output(
    output: &ScopedFile,
    expected: &[u8],
) -> Result<(), ProductionPackageError> {
    match output.inspect(ARCHIVE_OUTPUT_LIMIT) {
        Ok(Some(actual)) if actual == expected => Ok(()),
        _ => Err(failure(ProductionPackageErrorId::OutputFailed)),
    }
}

fn restore_archive_output(
    output: &ScopedFile,
    published: &[u8],
    previous: Option<&[u8]>,
) -> Result<(), ProductionPackageError> {
    match output.apply(Some(&sha256(published)), previous) {
        Ok(true) => Ok(()),
        Ok(false) | Err(_) => Err(failure(ProductionPackageErrorId::OutputFailed)),
    }
}
