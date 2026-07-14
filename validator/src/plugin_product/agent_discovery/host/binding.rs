use super::{HostAgentAuthorityRequest, HostAgentAuthorityTransaction, ReadOnlyEffectRequest};
use crate::plugin_product::agent_discovery::error::{AgentDiscoveryError, AgentDiscoveryErrorId};
use crate::plugin_product::agent_discovery::model::{AgentAuthorityLayer, MAX_CATALOG_BYTES};
use crate::plugin_product::agent_discovery::protocol_codec::{
    AgentProtocolCodecRequest, encode_digest,
};
use crate::plugin_product::agent_discovery::source::SourceAgentCatalog;

pub(crate) struct BoundHostAgentAuthorityTransaction<'a> {
    transaction: &'a mut dyn HostAgentAuthorityTransaction,
    start_generation: u64,
}

impl<'a> BoundHostAgentAuthorityTransaction<'a> {
    pub(crate) fn bind(
        transaction: &'a mut dyn HostAgentAuthorityTransaction,
        request: &HostAgentAuthorityRequest,
    ) -> Result<Self, AgentDiscoveryError> {
        let bound = Self {
            start_generation: transaction.start_generation(),
            transaction,
        };
        bound.require_current(request)?;
        Ok(bound)
    }

    pub(crate) fn require_current(
        &self,
        request: &HostAgentAuthorityRequest,
    ) -> Result<(), AgentDiscoveryError> {
        if self.transaction.provenance_sha256() != request.provenance_sha256()
            || self.transaction.project_root_sha256() != request.project_root_sha256()
            || self.transaction.candidate_id() != request.candidate_id()
            || self.transaction.session_id() != request.session_id()
            || self.transaction.session_issuance_sha256() != request.session_issuance_sha256()
            || self.transaction.observation_nonce_sha256() != request.observation_nonce_sha256()
            || self.transaction.start_generation() != self.start_generation
            || self
                .transaction
                .current_generation()
                .map_err(|_| unavailable())?
                != self.start_generation
        {
            return Err(changed());
        }
        Ok(())
    }

    pub(crate) fn capture_raw(
        &mut self,
        request: &HostAgentAuthorityRequest,
    ) -> Result<Vec<(AgentAuthorityLayer, Vec<u8>)>, AgentDiscoveryError> {
        self.require_current(request)?;
        let mut result = Vec::with_capacity(AgentAuthorityLayer::ALL.len());
        for layer in AgentAuthorityLayer::ALL {
            self.require_current(request)?;
            let bytes = self
                .transaction
                .read_catalog(layer, MAX_CATALOG_BYTES)
                .map_err(|_| unavailable())?
                .ok_or_else(unavailable)?;
            if bytes.len() > MAX_CATALOG_BYTES {
                return Err(AgentDiscoveryError::new(
                    AgentDiscoveryErrorId::InputTooLarge,
                ));
            }
            result.push((layer, bytes));
        }
        self.require_current(request)?;
        Ok(result)
    }

    pub(crate) fn enforce_effects(
        &mut self,
        source: &SourceAgentCatalog,
        binding_sha256: &str,
        request: &HostAgentAuthorityRequest,
    ) -> Result<String, AgentDiscoveryError> {
        let mut rows = Vec::new();
        for role in source.canonical_agents() {
            self.require_current(request)?;
            let probe = ReadOnlyEffectRequest::issue(
                binding_sha256,
                role.name(),
                role.descriptor_sha256(),
                request.observation_nonce_sha256(),
            )?;
            let enforcement = self
                .transaction
                .enforce_read_only(&probe)
                .map_err(|_| unavailable())?;
            enforcement.require_exact(&probe)?;
            rows.push(enforcement);
        }
        self.require_current(request)?;
        encode_digest(AgentProtocolCodecRequest::ReadOnlyEffectSet { rows: &rows })
            .map(|response| response.sha256())
            .map_err(|_| invalid_binding())
    }
}

fn invalid_binding() -> AgentDiscoveryError {
    AgentDiscoveryError::new(AgentDiscoveryErrorId::InvalidBinding)
}

fn unavailable() -> AgentDiscoveryError {
    AgentDiscoveryError::new(AgentDiscoveryErrorId::ObservationUnavailable)
}

fn changed() -> AgentDiscoveryError {
    AgentDiscoveryError::new(AgentDiscoveryErrorId::ObservationChanged)
}
