pub fn publish_registry_file(
    file: &mut crate::distribution::filesystem::ScopedFile,
    binding: &JourneyBinding,
    host: &HostCapabilityDeclaration,
    registered: bool,
    visible: bool,
) -> Result<(), DistributionError> {
    if file.root_id() != binding.home_id()
        || file.relative_path() != APP_REGISTRY_PATH
        || host.state(Capability::AppRegistry) != HostCapabilityState::Supported
    {
        return Err(error(DistributionErrorId::ProvenanceMismatch));
    }
    host.ensure_binding(binding)?;
    let bytes = registry_document(binding, registered, visible)?;
    let current = file.inspect(REGISTRY_LIMIT)?;
    if current.as_deref() != Some(bytes.as_slice())
        && !file.apply(current.as_deref().map(sha256).as_deref(), Some(&bytes))?
    {
        return Err(error(DistributionErrorId::InstallConflict));
    }
    let observed = file
        .inspect(REGISTRY_LIMIT)?
        .ok_or_else(|| error(DistributionErrorId::ObjectUnavailable))?;
    if observed != bytes {
        return Err(error(DistributionErrorId::ObjectChanged));
    }
    Ok(())
}
