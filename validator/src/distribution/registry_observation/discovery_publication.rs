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
    observe_discovery_file(file, binding, host).map(|_| ())
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
    let before = reader
        .inspect(REGISTRY_LIMIT)?
        .ok_or_else(|| error(DistributionErrorId::ObjectUnavailable))?;
    let document: DiscoveryDocument = json::parse(&before, REGISTRY_LIMIT)
        .map_err(|_| error(DistributionErrorId::ProvenanceMismatch))?;
    let source = binding.package().source();
    if document.schema != "harness-ultragoal.isolated-discovery.v1"
        || document.context_id != source.context_id()
        || document.candidate_id != source.candidate_id()
        || document.binding_sha256 != binding.binding_sha256()
        || document.package_sha256 != binding.package().archive_sha256()
        || reader.inspect(REGISTRY_LIMIT)?.as_deref() != Some(before.as_slice())
    {
        return Err(error(DistributionErrorId::ProvenanceMismatch));
    }
    Ok(DiscoveryObservation {
        context_id: source.context_id().into(),
        candidate_id: source.candidate_id().into(),
        verdict: LayerVerdict::Verified,
        discovery_verdict: DiscoveryVerdict::Visible,
        binding_sha256: Some(binding.binding_sha256().into()),
        observation_sha256: Some(sha256(&before)),
        confined_file_observation: true,
    })
}
