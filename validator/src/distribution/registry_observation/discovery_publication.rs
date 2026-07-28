#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DiscoveryDocument {
    schema: String,
    context_id: String,
    candidate_id: String,
    binding_sha256: String,
    package_sha256: String,
}

pub fn publish_discovery_file(
    file: &crate::distribution::filesystem::ScopedFile,
    binding: &JourneyBinding,
    host: &HostCapabilityDeclaration,
) -> Result<(), DistributionError> {
    if file.root_id() != binding.home_id()
        || file.relative_path() != DISCOVERY_PATH
        || host.state(Capability::Discovery) != HostCapabilityState::Supported
    {
        return Err(error(DistributionErrorId::ProvenanceMismatch));
    }
    host.ensure_binding(binding)?;
    let source = binding.package().source();
    let bytes = serde_json::to_vec(&DiscoveryDocument {
        schema: "harness-ultragoal.isolated-discovery.v1".into(),
        context_id: source.context_id().into(),
        candidate_id: source.candidate_id().into(),
        binding_sha256: binding.binding_sha256().into(),
        package_sha256: binding.package().archive_sha256().into(),
    })
    .map_err(|_| error(DistributionErrorId::InvalidSpec))?;
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

pub fn observe_discovery_file(
    reader: &crate::distribution::filesystem::ScopedFile,
    binding: &JourneyBinding,
    host: &HostCapabilityDeclaration,
) -> Result<DiscoveryObservation, DistributionError> {
    if reader.root_id() != binding.home_id()
        || reader.relative_path() != DISCOVERY_PATH
        || host.state(Capability::Discovery) != HostCapabilityState::Supported
    {
        return Err(error(DistributionErrorId::ProvenanceMismatch));
    }
    host.ensure_binding(binding)?;
    if reader.inspect(REGISTRY_LIMIT)?.is_some() {
        return Err(error(DistributionErrorId::ProvenanceMismatch));
    }
    Err(error(DistributionErrorId::ObjectUnavailable))
}
