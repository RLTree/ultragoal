use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use crate::distribution::host_capability::{
    HostCapabilityDeclaration, HostCapabilityState, JourneyBinding,
};
use crate::distribution::json;
use crate::distribution::model::{Capability, DistributionReport, Layer, LayerVerdict};
use crate::distribution::reader::sha256;
use crate::plugin_manifest::Version;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const REGISTRY_LIMIT: usize = 4 * 1024 * 1024;

pub trait RegistryReader {
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
    observation_sha256: Option<String>,
    verdict: AppRegistryVerdict,
}

impl AppRegistryObservation {
    pub fn verdict(&self) -> AppRegistryVerdict {
        self.verdict
    }
    pub fn observation_sha256(&self) -> Option<&str> {
        self.observation_sha256.as_deref()
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
    reader: &mut impl RegistryReader,
    binding: &JourneyBinding,
    host: &HostCapabilityDeclaration,
) -> Result<RegistryObservations, DistributionError> {
    let before = reader
        .read_registry(REGISTRY_LIMIT)
        .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
    let app_registry = observe_app_registry(before.as_deref(), binding, host)?;
    let discovery = observe_discovery(before.as_deref(), binding, host)?;
    let after = reader
        .read_registry(REGISTRY_LIMIT)
        .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
    if before != after {
        return Err(error(DistributionErrorId::ObjectChanged));
    }
    Ok(RegistryObservations {
        app_registry,
        discovery,
    })
}

pub fn observe_app_registry(
    bytes: Option<&[u8]>,
    binding: &JourneyBinding,
    host: &HostCapabilityDeclaration,
) -> Result<AppRegistryObservation, DistributionError> {
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
        observation_sha256: bytes.map(sha256),
        verdict,
    })
}

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
