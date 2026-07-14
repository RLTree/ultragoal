#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DiscoveryDocument {
    schema: String,
    context_id: String,
    candidate_id: String,
    home_id: String,
    project_id: String,
    host_id: String,
    capability_sha256: String,
    binding_sha256: String,
    entries: Vec<DiscoveryEntry>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DiscoveryEntry {
    plugin_id: String,
    version: String,
    package_sha256: String,
    installed_tree_sha256: String,
    visible: bool,
}

pub fn discovery_document(
    binding: &JourneyBinding,
    visible: bool,
) -> Result<Vec<u8>, DistributionError> {
    let source = binding.package().source();
    serde_json::to_vec(&DiscoveryDocument {
        schema: "harness-ultragoal.host-discovery.v1".into(),
        context_id: source.context_id().into(),
        candidate_id: source.candidate_id().into(),
        home_id: binding.home_id().into(),
        project_id: binding.project_id().into(),
        host_id: binding.host_id().into(),
        capability_sha256: binding.capability_sha256().into(),
        binding_sha256: binding.binding_sha256().into(),
        entries: vec![DiscoveryEntry {
            plugin_id: source.plugin_id().into(),
            version: source.version().into(),
            package_sha256: binding.package().archive_sha256().into(),
            installed_tree_sha256: binding.package().tree_sha256().into(),
            visible,
        }],
    })
    .map_err(|_| error(DistributionErrorId::InvalidSpec))
}

pub fn observe_discovery_file(
    reader: &mut crate::distribution::filesystem::ScopedFile,
    binding: &JourneyBinding,
    host: &HostCapabilityDeclaration,
) -> Result<DiscoveryObservation, DistributionError> {
    if reader.root_id() != binding.home_id() || reader.relative_path() != DISCOVERY_PATH {
        return Err(error(DistributionErrorId::ProvenanceMismatch));
    }
    host.ensure_binding(binding)?;
    let before = reader
        .inspect(REGISTRY_LIMIT)
        .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
    let mut discovery = observe_discovery_inner(before.as_deref(), binding, host)?;
    let after = reader
        .inspect(REGISTRY_LIMIT)
        .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
    if before != after {
        return Err(error(DistributionErrorId::ObjectChanged));
    }
    discovery.confined_file_observation = true;
    Ok(discovery)
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
            let visible = validate_discovery(
                bytes.ok_or_else(|| error(DistributionErrorId::ObjectUnavailable))?,
                binding,
            )?;
            if visible {
                (LayerVerdict::Verified, DiscoveryVerdict::Visible)
            } else {
                (
                    LayerVerdict::Contradicted,
                    DiscoveryVerdict::RegisteredHidden,
                )
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

fn validate_discovery(bytes: &[u8], binding: &JourneyBinding) -> Result<bool, DistributionError> {
    let document: DiscoveryDocument = json::parse(bytes, REGISTRY_LIMIT)?;
    let source = binding.package().source();
    if document.schema != "harness-ultragoal.host-discovery.v1"
        || document.context_id != source.context_id()
        || document.candidate_id != source.candidate_id()
        || document.home_id != binding.home_id()
        || document.project_id != binding.project_id()
        || document.host_id != binding.host_id()
        || document.capability_sha256 != binding.capability_sha256()
        || document.binding_sha256 != binding.binding_sha256()
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
    Ok(row.visible)
}
