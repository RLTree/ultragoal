use super::*;

impl DarwinHostTransactionAdapter {
    pub(super) fn observe_surface(
        &self,
        plan: &DarwinHostTransactionPlan,
        surface: DarwinHostSurface,
    ) -> Result<DarwinSurfaceObservation, DarwinHostError> {
        let Some(raw) = self.read_surface(surface)? else {
            return Ok(DarwinSurfaceObservation::absent(surface));
        };
        let decoded = match decode_surface(surface, &raw.bytes) {
            Ok(record) => record,
            Err(problem) => {
                let status = match problem.id() {
                    DarwinHostErrorId::CrossSurfaceSubstitution => {
                        DarwinSurfaceStatus::CrossSurface
                    }
                    DarwinHostErrorId::PackageSubstitution => DarwinSurfaceStatus::Substituted,
                    DarwinHostErrorId::ObjectTooLarge => return Err(problem),
                    _ => DarwinSurfaceStatus::Dirty,
                };
                return DarwinSurfaceObservation::conflict(
                    surface,
                    status,
                    raw.tree_sha256,
                    None,
                    None,
                );
            }
        };
        let exact_target = plan.desired_by_surface.get(&surface) == Some(&raw.bytes);
        let exact_prior = plan.prior_by_surface.get(&surface) == Some(&raw.bytes);
        let status = if exact_target {
            DarwinSurfaceStatus::Verified
        } else if exact_prior
            || decoded.package.plugin_id == plan.target.plugin_id
                && decoded.package.candidate_id == plan.target.candidate_id
                && decoded.package.version != plan.target.version
        {
            DarwinSurfaceStatus::Stale
        } else if decoded.surface != surface {
            DarwinSurfaceStatus::CrossSurface
        } else if decoded.root_id != self.root_id
            || decoded.marketplace != SUPPORTED_MARKETPLACE
            || decoded.package != plan.target
        {
            DarwinSurfaceStatus::Substituted
        } else {
            DarwinSurfaceStatus::Dirty
        };
        if status == DarwinSurfaceStatus::Verified {
            DarwinSurfaceObservation::verified(
                surface,
                raw.tree_sha256,
                decoded.package.candidate_id,
                decoded.package.version,
            )
        } else {
            DarwinSurfaceObservation::conflict(
                surface,
                status,
                raw.tree_sha256,
                Some(decoded.package.candidate_id),
                Some(decoded.package.version),
            )
        }
    }

    pub(super) fn read_all_surfaces(&self) -> Result<[Option<RawTree>; 7], DarwinHostError> {
        DarwinHostSurface::ALL
            .map(|surface| self.read_surface(surface))
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?
            .try_into()
            .map_err(|_| DarwinHostError::new(DarwinHostErrorId::EffectFailed))
    }

    pub(super) fn read_surface(
        &self,
        surface: DarwinHostSurface,
    ) -> Result<Option<RawTree>, DarwinHostError> {
        self.read_tree(surface.relative_path(), Some(surface), SURFACE_LIMIT)
    }

    pub(super) fn read_tree(
        &self,
        relative: &str,
        surface: Option<DarwinHostSurface>,
        maximum: usize,
    ) -> Result<Option<RawTree>, DarwinHostError> {
        let rows = ScopedTree::new(self.root.clone(), relative)
            .and_then(|row| row.inspect(TREE_ENTRY_LIMIT, maximum))
            .map_err(|error| map_surface_distribution(error, surface))?;
        let Some(rows) = rows else {
            return Ok(None);
        };
        if rows.len() != 1 || rows[0].path() != "record" {
            return Err(surface_error(DarwinHostErrorId::SurfaceConflict, surface));
        }
        if rows[0].mode() != 0o644 {
            return Err(surface_error(DarwinHostErrorId::UnsafeObject, surface));
        }
        let tree_sha256 =
            tree_sha256(&rows).map_err(|error| map_surface_distribution(error, surface))?;
        Ok(Some(RawTree {
            bytes: rows[0].bytes().to_vec(),
            tree_sha256,
        }))
    }
}
