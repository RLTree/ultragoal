use super::error::{AgentDiscoveryError, AgentDiscoveryErrorId};
use super::model::{AgentAuthorityLayer, CanonicalAgentObservation};
use super::session::AgentDiscoverySession;
use super::source::SourceAgentCatalog;
use super::supported::{SupportedHostAgentAuthorityReader, SupportedHostAgentRoots};
use std::path::{Path, PathBuf};

pub(crate) struct LocalAgentAuthorityRequest<'a> {
    pub(crate) source_root: &'a Path,
    pub(crate) package_root: PathBuf,
    pub(crate) installed_root: PathBuf,
    pub(crate) cache_family_root: PathBuf,
    pub(crate) global_root: PathBuf,
    pub(crate) project_root: PathBuf,
    pub(crate) candidate_id: &'a str,
    pub(crate) session_id: &'a str,
}

pub(crate) struct LocalAgentRoleObservation {
    name: String,
    package_matches: bool,
    installed_matches: bool,
    cache_matches: bool,
    global_matches: bool,
    project_matches: bool,
}

impl LocalAgentRoleObservation {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }
    pub(crate) fn package_matches(&self) -> bool {
        self.package_matches
    }
    pub(crate) fn installed_matches(&self) -> bool {
        self.installed_matches
    }
    pub(crate) fn cache_matches(&self) -> bool {
        self.cache_matches
    }
    pub(crate) fn global_matches(&self) -> bool {
        self.global_matches
    }
    pub(crate) fn project_matches(&self) -> bool {
        self.project_matches
    }
}

pub(crate) struct LocalAgentAuthorityObservation {
    source_catalog_sha256: String,
    binding_sha256: String,
    roles: Vec<LocalAgentRoleObservation>,
}

impl LocalAgentAuthorityObservation {
    pub(crate) fn source_catalog_sha256(&self) -> &str {
        &self.source_catalog_sha256
    }
    pub(crate) fn binding_sha256(&self) -> &str {
        &self.binding_sha256
    }
    pub(crate) fn roles(&self) -> &[LocalAgentRoleObservation] {
        &self.roles
    }
}

pub(crate) fn observe_local_authority(
    request: LocalAgentAuthorityRequest<'_>,
) -> Result<LocalAgentAuthorityObservation, AgentDiscoveryError> {
    let source = SourceAgentCatalog::capture(
        request.source_root,
        request.candidate_id,
        request.session_id,
    )?;
    let session = AgentDiscoverySession::bind(source.clone())?;
    let roots = SupportedHostAgentRoots::new(
        request.package_root,
        request.installed_root,
        request.cache_family_root.join(source.plugin_version()),
        request.global_root,
        request.project_root,
    );
    let mut reader = SupportedHostAgentAuthorityReader::open(source.clone(), roots)?;
    let eligibility = session.verify(&mut reader)?;
    if eligibility.package_version() != source.plugin_version()
        || !super::filesystem::valid_sha256(eligibility.sandbox_effect_sha256())
        || eligibility.new_session_observed()
        || eligibility.route_eligible()
        || eligibility.has_claim_effect()
    {
        return Err(AgentDiscoveryError::new(
            AgentDiscoveryErrorId::InvalidBinding,
        ));
    }
    let roles = source
        .canonical_agents()
        .into_iter()
        .map(|agent| role_observation(&agent, eligibility.layers()))
        .collect();
    Ok(LocalAgentAuthorityObservation {
        source_catalog_sha256: eligibility.source_catalog_sha256().to_owned(),
        binding_sha256: eligibility.binding_sha256().to_owned(),
        roles,
    })
}

fn role_observation(
    source: &CanonicalAgentObservation,
    layers: &[super::model::AgentLayerObservation],
) -> LocalAgentRoleObservation {
    let matches = |layer| layer_matches(layers, layer, source);
    LocalAgentRoleObservation {
        name: source.name().to_owned(),
        package_matches: matches(AgentAuthorityLayer::Package),
        installed_matches: matches(AgentAuthorityLayer::Installed),
        cache_matches: matches(AgentAuthorityLayer::Cache),
        global_matches: matches(AgentAuthorityLayer::Global),
        project_matches: matches(AgentAuthorityLayer::Discovery),
    }
}

fn layer_matches(
    layers: &[super::model::AgentLayerObservation],
    layer: AgentAuthorityLayer,
    source: &CanonicalAgentObservation,
) -> bool {
    layers
        .iter()
        .find(|observation| observation.layer() == layer)
        .and_then(|observation| {
            observation
                .canonical_agents()
                .iter()
                .find(|agent| agent.name() == source.name())
        })
        .is_some_and(|agent| {
            agent.manifest_path() == source.manifest_path()
                && agent.descriptor_sha256() == source.descriptor_sha256()
        })
}
