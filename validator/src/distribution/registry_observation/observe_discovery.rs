pub fn observe_supported_host_discovery(
    installed: &mut crate::distribution::filesystem::ScopedFile,
    binding: &JourneyBinding,
    host: &HostCapabilityDeclaration,
    install: &crate::distribution::install::InstallSnapshot,
    app_registry: &AppRegistryObservation,
) -> Result<DiscoveryObservation, DistributionError> {
    if installed.root_id() != binding.home_id()
        || installed.relative_path() != SUPPORTED_INSTALL_TARGET
        || host.state(Capability::Discovery) != HostCapabilityState::Supported
        || host.state(Capability::AppRegistry) != HostCapabilityState::Supported
        || install.context_id() != binding.package().source().context_id()
        || install.candidate_id() != binding.package().source().candidate_id()
        || install.package_sha256() != binding.package().archive_sha256()
        || install.target_id() != sha256(SUPPORTED_INSTALL_TARGET.as_bytes())
        || app_registry.context_id() != binding.package().source().context_id()
        || app_registry.candidate_id() != binding.package().source().candidate_id()
        || app_registry.binding_sha256() != binding.binding_sha256()
        || app_registry.verdict() != AppRegistryVerdict::Verified
        || !app_registry.is_confined_file_observation()
        || app_registry.observation_sha256().is_none()
    {
        return Err(error(DistributionErrorId::ProvenanceMismatch));
    }
    host.ensure_binding(binding)?;
    Err(error(DistributionErrorId::ObjectUnavailable))
}

pub fn observe_discovery(
    bytes: Option<&[u8]>,
    binding: &JourneyBinding,
    host: &HostCapabilityDeclaration,
) -> Result<DiscoveryObservation, DistributionError> {
    host.ensure_binding(binding)?;
    match host.state(Capability::Discovery) {
        HostCapabilityState::Unsupported | HostCapabilityState::Absent if bytes.is_none() => {
            observe_discovery_inner(None, binding, host)
        }
        _ => Err(error(DistributionErrorId::ProvenanceMismatch)),
    }
}

fn observe_discovery_inner(
    bytes: Option<&[u8]>,
    binding: &JourneyBinding,
    host: &HostCapabilityDeclaration,
) -> Result<DiscoveryObservation, DistributionError> {
    host.ensure_binding(binding)?;
    let (verdict, discovery_verdict) = match host.state(Capability::Discovery) {
        HostCapabilityState::Supported => {
            let _ = bytes.ok_or_else(|| error(DistributionErrorId::ObjectUnavailable))?;
            return Err(error(DistributionErrorId::ProvenanceMismatch));
        }
        HostCapabilityState::Unsupported if bytes.is_none() => {
            (LayerVerdict::Unavailable, DiscoveryVerdict::Unsupported)
        }
        HostCapabilityState::Absent if bytes.is_none() => {
            (LayerVerdict::Unavailable, DiscoveryVerdict::Absent)
        }
        _ => return Err(error(DistributionErrorId::CapabilityMismatch)),
    };
    Ok(DiscoveryObservation {
        context_id: binding.package().source().context_id().into(),
        candidate_id: binding.package().source().candidate_id().into(),
        verdict,
        discovery_verdict,
        binding_sha256: Some(binding.binding_sha256().into()),
        observation_sha256: bytes.map(sha256),
        confined_file_observation: false,
    })
}
