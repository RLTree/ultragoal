const REGISTRY_LIMIT: usize = 4 * 1024 * 1024;
const APP_REGISTRY_PATH: &str = "app/registry.json";
const DISCOVERY_PATH: &str = "host/discovery.json";

pub(crate) trait RegistryReader {
    fn read_registry(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()>;
}

impl RegistryReader for crate::distribution::filesystem::ScopedFile {
    fn read_registry(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        self.inspect(maximum).map_err(|_| ())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AppRegistryVerdict {
    Verified,
    Unsupported,
    Absent,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AppRegistryObservation {
    context_id: String,
    candidate_id: String,
    binding_sha256: String,
    observation_sha256: Option<String>,
    verdict: AppRegistryVerdict,
    #[serde(skip)]
    confined_file_observation: bool,
}

impl AppRegistryObservation {
    pub fn verdict(&self) -> AppRegistryVerdict {
        self.verdict
    }
    pub fn observation_sha256(&self) -> Option<&str> {
        self.observation_sha256.as_deref()
    }
    pub fn context_id(&self) -> &str {
        &self.context_id
    }
    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }
    pub fn binding_sha256(&self) -> &str {
        &self.binding_sha256
    }
    pub(crate) const fn is_confined_file_observation(&self) -> bool {
        self.confined_file_observation
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiscoveryVerdict {
    Visible,
    RegisteredHidden,
    Unsupported,
    Absent,
    DefinitionOnly,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DiscoveryObservation {
    context_id: String,
    candidate_id: String,
    verdict: LayerVerdict,
    discovery_verdict: DiscoveryVerdict,
    binding_sha256: Option<String>,
    observation_sha256: Option<String>,
    #[serde(skip)]
    confined_file_observation: bool,
}

impl DiscoveryObservation {
    pub fn from_report(report: &DistributionReport) -> Self {
        Self {
            context_id: report.context_id().to_owned(),
            candidate_id: report.candidate_id().to_owned(),
            verdict: report.layer(Layer::Discovery).verdict(),
            discovery_verdict: DiscoveryVerdict::DefinitionOnly,
            binding_sha256: None,
            observation_sha256: None,
            confined_file_observation: false,
        }
    }
    pub const fn verdict(&self) -> LayerVerdict {
        self.verdict
    }
    pub const fn discovery_verdict(&self) -> DiscoveryVerdict {
        self.discovery_verdict
    }
    pub fn is_current_visible(&self) -> bool {
        self.discovery_verdict == DiscoveryVerdict::Visible
    }
    pub fn observation_sha256(&self) -> Option<&str> {
        self.observation_sha256.as_deref()
    }
    pub fn context_id(&self) -> &str {
        &self.context_id
    }
    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }
    pub fn binding_sha256(&self) -> Option<&str> {
        self.binding_sha256.as_deref()
    }
    pub(crate) const fn is_confined_file_observation(&self) -> bool {
        self.confined_file_observation
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryObservations {
    pub app_registry: AppRegistryObservation,
    pub discovery: DiscoveryObservation,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RegistryDocument {
    schema: String,
    context_id: String,
    candidate_id: String,
    home_id: String,
    project_id: String,
    host_id: String,
    capability_sha256: String,
    entries: Vec<RegistryEntry>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RegistryEntry {
    plugin_id: String,
    version: String,
    package_sha256: String,
    installed_tree_sha256: String,
    registered: bool,
    visible: bool,
}

pub fn registry_document(
    binding: &JourneyBinding,
    registered: bool,
    visible: bool,
) -> Result<Vec<u8>, DistributionError> {
    if visible && !registered {
        return Err(error(DistributionErrorId::InvalidSpec));
    }
    let source = binding.package().source();
    serde_json::to_vec(&RegistryDocument {
        schema: "harness-ultragoal.isolated-app-registry.v1".into(),
        context_id: source.context_id().into(),
        candidate_id: source.candidate_id().into(),
        home_id: binding.home_id().into(),
        project_id: binding.project_id().into(),
        host_id: binding.host_id().into(),
        capability_sha256: binding.capability_sha256().into(),
        entries: vec![RegistryEntry {
            plugin_id: source.plugin_id().into(),
            version: source.version().into(),
            package_sha256: binding.package().archive_sha256().into(),
            installed_tree_sha256: binding.package().tree_sha256().into(),
            registered,
            visible,
        }],
    })
    .map_err(|_| error(DistributionErrorId::InvalidSpec))
}

pub fn observe_registry_file(
    reader: &mut crate::distribution::filesystem::ScopedFile,
    binding: &JourneyBinding,
    host: &HostCapabilityDeclaration,
) -> Result<AppRegistryObservation, DistributionError> {
    if reader.root_id() != binding.home_id() || reader.relative_path() != APP_REGISTRY_PATH {
        return Err(error(DistributionErrorId::ProvenanceMismatch));
    }
    host.ensure_binding(binding)?;
    observe_registry_reader_inner(reader, binding, host, true)
}

#[cfg(test)]
pub(crate) fn observe_registry_reader(
    reader: &mut impl RegistryReader,
    binding: &JourneyBinding,
    host: &HostCapabilityDeclaration,
) -> Result<AppRegistryObservation, DistributionError> {
    observe_registry_reader_inner(reader, binding, host, false)
}

fn observe_registry_reader_inner(
    reader: &mut impl RegistryReader,
    binding: &JourneyBinding,
    host: &HostCapabilityDeclaration,
    confined_file_observation: bool,
) -> Result<AppRegistryObservation, DistributionError> {
    host.ensure_binding(binding)?;
    let before = reader
        .read_registry(REGISTRY_LIMIT)
        .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
    let mut app_registry = observe_app_registry(before.as_deref(), binding, host)?;
    let after = reader
        .read_registry(REGISTRY_LIMIT)
        .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
    if before != after {
        return Err(error(DistributionErrorId::ObjectChanged));
    }
    app_registry.confined_file_observation = confined_file_observation;
    Ok(app_registry)
}

pub fn observe_app_registry(
    bytes: Option<&[u8]>,
    binding: &JourneyBinding,
    host: &HostCapabilityDeclaration,
) -> Result<AppRegistryObservation, DistributionError> {
    host.ensure_binding(binding)?;
    let verdict = match host.state(Capability::AppRegistry) {
        HostCapabilityState::Supported => {
            validate_registry(
                bytes.ok_or_else(|| error(DistributionErrorId::ObjectUnavailable))?,
                binding,
            )?;
            AppRegistryVerdict::Verified
        }
        HostCapabilityState::Unsupported if bytes.is_none() => AppRegistryVerdict::Unsupported,
        HostCapabilityState::Absent if bytes.is_none() => AppRegistryVerdict::Absent,
        _ => return Err(error(DistributionErrorId::CapabilityMismatch)),
    };
    Ok(AppRegistryObservation {
        context_id: binding.package().source().context_id().into(),
        candidate_id: binding.package().source().candidate_id().into(),
        binding_sha256: binding.binding_sha256().into(),
        observation_sha256: bytes.map(sha256),
        verdict,
        confined_file_observation: false,
    })
}
