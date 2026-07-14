mod authority;
#[path = "../catalog_codec/mod.rs"]
mod catalog;

use super::report::{ReportState, set_generation};
use super::roots::{LayerFiles, SupportedRootSet};
use super::{changed, invalid_binding};
use crate::plugin_product::agent_discovery::error::AgentDiscoveryError;
use crate::plugin_product::agent_discovery::filesystem::digest;
use crate::plugin_product::agent_discovery::host::HostAgentAuthorityRequest;
use crate::plugin_product::agent_discovery::model::AgentAuthorityLayer;
use crate::plugin_product::agent_discovery::source::SourceAgentCatalog;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

pub(super) struct SupportedTransaction {
    source: SourceAgentCatalog,
    roots: SupportedRootSet,
    snapshots: BTreeMap<AgentAuthorityLayer, LayerFiles>,
    provenance_sha256: String,
    project_root_sha256: String,
    candidate_id: String,
    session_id: String,
    session_issuance_sha256: String,
    observation_nonce_sha256: String,
    generation_sha256: String,
    start_generation: u64,
    report: Arc<Mutex<ReportState>>,
}

impl SupportedTransaction {
    pub(super) fn open(
        source: SourceAgentCatalog,
        roots: SupportedRootSet,
        request: &HostAgentAuthorityRequest,
        report: Arc<Mutex<ReportState>>,
    ) -> Result<Self, AgentDiscoveryError> {
        source.revalidate()?;
        roots.revalidate()?;
        let mut snapshots = BTreeMap::new();
        for layer in AgentAuthorityLayer::ALL {
            snapshots.insert(layer, roots.capture(layer)?);
        }
        let generation_sha256 = catalog::generation_sha256(&source, &roots, &snapshots)?;
        let start_generation =
            u64::from_str_radix(&generation_sha256[7..23], 16).map_err(|_| invalid_binding())?;
        set_generation(&report, generation_sha256.clone());
        Ok(Self {
            source,
            roots,
            snapshots,
            provenance_sha256: request.provenance_sha256().to_owned(),
            project_root_sha256: request.project_root_sha256().to_owned(),
            candidate_id: request.candidate_id().to_owned(),
            session_id: request.session_id().to_owned(),
            session_issuance_sha256: request.session_issuance_sha256().to_owned(),
            observation_nonce_sha256: request.observation_nonce_sha256().to_owned(),
            generation_sha256,
            start_generation,
            report,
        })
    }

    fn recapture(&self, layer: AgentAuthorityLayer) -> Result<LayerFiles, AgentDiscoveryError> {
        self.source.revalidate()?;
        let current = self.roots.capture(layer)?;
        if self.snapshots.get(&layer) != Some(&current) {
            return Err(changed());
        }
        self.source.revalidate()?;
        Ok(current)
    }

    fn revalidate_all(&self) -> Result<(), AgentDiscoveryError> {
        self.source.revalidate()?;
        self.roots.revalidate()?;
        for layer in AgentAuthorityLayer::ALL {
            self.recapture(layer)?;
        }
        if catalog::generation_sha256(&self.source, &self.roots, &self.snapshots)?
            != self.generation_sha256
        {
            return Err(changed());
        }
        self.source.revalidate()
    }
}

fn catalog_digest(bytes: &[u8]) -> String {
    digest(bytes)
}
