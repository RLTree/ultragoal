pub fn observe_discovery(
    bytes: Option<&[u8]>,
    binding: &JourneyBinding,
    host: &HostCapabilityDeclaration,
) -> Result<DiscoveryObservation, DistributionError> {
    let (verdict, discovery_verdict) = match host.state(Capability::Discovery) {
        HostCapabilityState::Supported => {
            let row = validate_registry(
                bytes.ok_or_else(|| error(DistributionErrorId::ObjectUnavailable))?,
                binding,
            )?;
            if row.registered && row.visible {
                (LayerVerdict::Verified, DiscoveryVerdict::Visible)
            } else if row.registered {
                (
                    LayerVerdict::Contradicted,
                    DiscoveryVerdict::RegisteredHidden,
                )
            } else {
                return Err(error(DistributionErrorId::InstallConflict));
            }
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

#[derive(Clone, Copy)]
struct RegistryState {
    registered: bool,
    visible: bool,
}

fn validate_registry(
    bytes: &[u8],
    binding: &JourneyBinding,
) -> Result<RegistryState, DistributionError> {
    let document: RegistryDocument = json::parse(bytes, REGISTRY_LIMIT)?;
    let source = binding.package().source();
    if document.schema != "harness-ultragoal.isolated-app-registry.v1"
        || document.context_id != source.context_id()
        || document.candidate_id != source.candidate_id()
        || document.home_id != binding.home_id()
        || document.project_id != binding.project_id()
        || document.host_id != binding.host_id()
        || document.capability_sha256 != binding.capability_sha256()
        || document.entries.is_empty()
        || document.entries.len() > 1024
    {
        return Err(error(DistributionErrorId::ProvenanceMismatch));
    }
    let mut identities = BTreeSet::new();
    for row in &document.entries {
        if row.plugin_id.is_empty()
            || Version::parse(&row.version).is_none()
            || !identities.insert((row.plugin_id.as_str(), row.version.as_str()))
            || row.visible && !row.registered
        {
            return Err(error(DistributionErrorId::InstallConflict));
        }
    }
    let mut matches = document
        .entries
        .iter()
        .filter(|row| row.plugin_id == source.plugin_id());
    let row = matches
        .next()
        .ok_or_else(|| error(DistributionErrorId::InstallConflict))?;
    if matches.next().is_some()
        || row.version != source.version()
        || row.package_sha256 != binding.package().archive_sha256()
        || row.installed_tree_sha256 != binding.package().tree_sha256()
    {
        return Err(error(DistributionErrorId::InstallConflict));
    }
    Ok(RegistryState {
        registered: row.registered,
        visible: row.visible,
    })
}
