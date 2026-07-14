use super::*;

pub(super) fn valid_target_binding(binding: &PackageBinding) -> bool {
    valid_package_binding(binding) && binding.version == SUPPORTED_VERSION
}

pub(super) fn valid_package_binding(binding: &PackageBinding) -> bool {
    binding.plugin_id == SUPPORTED_PLUGIN_ID
        && binding.archive_sha256 == binding.package_sha256
        && [
            &binding.context_id,
            &binding.candidate_id,
            &binding.catalog_id,
            &binding.accepted_inventory_sha256,
            &binding.inventory_sha256,
            &binding.tree_sha256,
            &binding.archive_sha256,
            &binding.package_sha256,
        ]
        .into_iter()
        .all(|value| valid_digest(value))
}

pub(super) fn valid_digest(value: &str) -> bool {
    Sha256Digest::valid(value)
}

pub(super) fn interrupt(
    hook: &mut impl FnMut(DarwinTestPoint) -> DarwinTestControl,
    point: DarwinTestPoint,
) -> Result<(), DarwinHostError> {
    if hook(point) == DarwinTestControl::Interrupt {
        Err(DarwinHostError::new(DarwinHostErrorId::Interrupted))
    } else {
        Ok(())
    }
}

pub(super) fn map_distribution(error: DistributionError) -> DarwinHostError {
    map_surface_distribution(error, None)
}

pub(super) fn map_surface_distribution(
    error: DistributionError,
    surface: Option<DarwinHostSurface>,
) -> DarwinHostError {
    let id = match error.id() {
        DistributionErrorId::InvalidPath => DarwinHostErrorId::ConfinementRejected,
        DistributionErrorId::ObjectTooLarge => DarwinHostErrorId::ObjectTooLarge,
        DistributionErrorId::UnsafeObject => DarwinHostErrorId::UnsafeObject,
        DistributionErrorId::ObjectChanged => DarwinHostErrorId::ObservationChanged,
        DistributionErrorId::InstallConflict => DarwinHostErrorId::SurfaceConflict,
        _ => DarwinHostErrorId::EffectFailed,
    };
    surface_error(id, surface)
}

pub(super) fn surface_error(
    id: DarwinHostErrorId,
    surface: Option<DarwinHostSurface>,
) -> DarwinHostError {
    surface.map_or_else(
        || DarwinHostError::new(id),
        |row| DarwinHostError::at(id, row),
    )
}
