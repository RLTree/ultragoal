pub fn verify_bound_surface_chain(
    rows: &[SurfaceIdentity],
    binding: &crate::distribution::host_capability::JourneyBinding,
) -> Result<(), DistributionError> {
    verify_surface_chain(rows)?;
    if rows.iter().any(|row| {
        row.package() != binding.package()
            || row.journey_binding_sha256() != Some(binding.binding_sha256())
    }) {
        return Err(error(DistributionErrorId::ProvenanceMismatch));
    }
    Ok(())
}

pub(crate) fn verify_surface_chain(rows: &[SurfaceIdentity]) -> Result<(), DistributionError> {
    if rows.len() != IdentitySurface::ALL.len() {
        return Err(error(DistributionErrorId::ProvenanceMismatch));
    }
    let mut seen = BTreeSet::new();
    let first = rows
        .first()
        .ok_or_else(|| error(DistributionErrorId::ProvenanceMismatch))?;
    let binding = first
        .journey_binding_sha256()
        .ok_or_else(|| error(DistributionErrorId::ProvenanceMismatch))?;
    for (expected, row) in IdentitySurface::ALL.into_iter().zip(rows) {
        row.validate()?;
        if row.surface != expected
            || !seen.insert(row.surface)
            || row.package != first.package
            || row.journey_binding_sha256() != Some(binding)
        {
            return Err(error(DistributionErrorId::ProvenanceMismatch));
        }
    }
    Ok(())
}

pub fn reject_stale_version_reuse(
    previous: &PackageIdentity,
    next: &PackageIdentity,
) -> Result<(), DistributionError> {
    previous.validate()?;
    next.validate()?;
    let previous_version = Version::parse(previous.source.version())
        .ok_or_else(|| error(DistributionErrorId::InvalidSpec))?;
    let next_version = Version::parse(next.source.version())
        .ok_or_else(|| error(DistributionErrorId::InvalidSpec))?;
    let precedence = next_version.precedence_cmp(&previous_version);
    if previous.source.plugin_id != next.source.plugin_id
        || precedence == Ordering::Less
        || (precedence == Ordering::Equal
            && (previous.tree_sha256 != next.tree_sha256
                || previous.archive_sha256 != next.archive_sha256))
    {
        return Err(error(DistributionErrorId::InstallConflict));
    }
    Ok(())
}
