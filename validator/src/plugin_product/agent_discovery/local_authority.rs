use super::error::{AgentDiscoveryError, AgentDiscoveryErrorId};
use super::model::{AgentAuthorityLayer, CanonicalAgentObservation};
use super::session::AgentDiscoverySession;
use super::source::SourceAgentCatalog;
use super::supported::{SupportedHostAgentAuthorityReader, SupportedHostAgentRoots};
use std::path::{Path, PathBuf};

/// The complete, explicit authority set consumed by repository agent adoption.
///
/// Callers must supply every root that contributes to the sealed observation;
/// this avoids ambient discovery and gives the public adapter one typed result
/// to project without inventing a second authority model.
pub(crate) struct AgentRepositoryAdoptionRequest<'a> {
    pub(crate) source_root: &'a Path,
    pub(crate) package_root: PathBuf,
    pub(crate) installed_root: PathBuf,
    pub(crate) cache_family_root: PathBuf,
    pub(crate) global_root: PathBuf,
    pub(crate) project_root: PathBuf,
    pub(crate) candidate_id: &'a str,
    pub(crate) session_id: &'a str,
}

pub(crate) struct AgentRepositoryRoleObservation {
    name: String,
    package_matches: bool,
    installed_matches: bool,
    cache_matches: bool,
    global_matches: bool,
    project_matches: bool,
}

impl AgentRepositoryRoleObservation {
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

pub(crate) struct AgentRepositoryAdoption {
    source_catalog_sha256: String,
    binding_sha256: String,
    fresh_session_observed: bool,
    route_eligible: bool,
    roles: Vec<AgentRepositoryRoleObservation>,
}

impl AgentRepositoryAdoption {
    pub(crate) fn source_catalog_sha256(&self) -> &str {
        &self.source_catalog_sha256
    }
    pub(crate) fn binding_sha256(&self) -> &str {
        &self.binding_sha256
    }
    pub fn fresh_session_observed(&self) -> bool {
        self.fresh_session_observed
    }

    /// This is a source-local route decision only. It is not host discovery,
    /// installation, runtime activation, or a claim effect.
    pub fn route_eligible(&self) -> bool {
        self.route_eligible
    }

    pub(crate) fn roles(&self) -> &[AgentRepositoryRoleObservation] {
        &self.roles
    }
}

/// Executes the one sealed, read-only agent/repository adoption transaction.
pub(crate) fn adopt_agent_repository(
    request: AgentRepositoryAdoptionRequest<'_>,
) -> Result<AgentRepositoryAdoption, AgentDiscoveryError> {
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
    Ok(AgentRepositoryAdoption {
        source_catalog_sha256: eligibility.source_catalog_sha256().to_owned(),
        binding_sha256: eligibility.binding_sha256().to_owned(),
        fresh_session_observed: eligibility.new_session_observed(),
        route_eligible: eligibility.route_eligible(),
        roles,
    })
}

fn role_observation(
    source: &CanonicalAgentObservation,
    layers: &[super::model::AgentLayerObservation],
) -> AgentRepositoryRoleObservation {
    let matches = |layer| layer_matches(layers, layer, source);
    AgentRepositoryRoleObservation {
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
