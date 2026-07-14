use super::*;

impl DarwinHostTransactionAdapter {
    #[cfg(test)]
    pub fn seed_candidate_for_test(
        &self,
        package: &PackageSnapshot,
    ) -> Result<(), DarwinHostError> {
        let binding = PackageBinding::from_snapshot(package)?;
        for surface in DarwinHostSurface::ALL {
            let bytes = encode_surface(surface, &self.root_id, &binding, package.archive())?;
            let replacement = surface_tree(bytes);
            if !self.compare_exchange_tree(
                surface.relative_path(),
                None,
                Some(&replacement),
                Some(surface),
            )? {
                return Err(DarwinHostError::at(
                    DarwinHostErrorId::SurfaceConflict,
                    surface,
                ));
            }
        }
        Ok(())
    }

    #[cfg(test)]
    pub fn replace_surface_for_test(
        &self,
        surface: DarwinHostSurface,
        package: &PackageSnapshot,
    ) -> Result<(), DarwinHostError> {
        let before = self.read_surface(surface)?;
        let binding = PackageBinding::from_snapshot(package)?;
        let bytes = encode_surface(surface, &self.root_id, &binding, package.archive())?;
        let replacement = surface_tree(bytes);
        if !self.compare_exchange_tree(
            surface.relative_path(),
            before.as_ref().map(|row| row.tree_sha256.as_str()),
            Some(&replacement),
            Some(surface),
        )? {
            return Err(DarwinHostError::at(
                DarwinHostErrorId::SurfaceConflict,
                surface,
            ));
        }
        Ok(())
    }

    #[cfg(test)]
    pub fn validate_rows_for_test(
        &self,
        surface: DarwinHostSurface,
        rows: &[TreeObject],
    ) -> Result<(), DarwinHostError> {
        if rows.len() != 1 || rows[0].path() != "record" || rows[0].mode() != 0o644 {
            return Err(DarwinHostError::at(
                DarwinHostErrorId::UnsafeObject,
                surface,
            ));
        }
        tree_sha256(rows)
            .map(|_| ())
            .map_err(|error| map_surface_distribution(error, Some(surface)))
    }
}
