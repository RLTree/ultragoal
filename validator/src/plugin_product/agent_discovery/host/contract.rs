use super::{HostAgentAuthorityRequest, ReadOnlyEffectEnforcement, ReadOnlyEffectRequest};
use crate::plugin_product::agent_discovery::model::AgentAuthorityLayer;

pub trait HostAgentAuthorityReader {
    fn with_transaction<T>(
        &mut self,
        request: &HostAgentAuthorityRequest,
        operation: impl FnOnce(
            Result<&mut dyn HostAgentAuthorityTransaction, HostAgentAuthorityTransactionError>,
        ) -> T,
    ) -> T;
}

pub trait HostAgentAuthorityTransaction {
    fn provenance_sha256(&self) -> &str;
    fn project_root_sha256(&self) -> &str;
    fn candidate_id(&self) -> &str;
    fn session_id(&self) -> &str;
    fn session_issuance_sha256(&self) -> &str;
    fn observation_nonce_sha256(&self) -> &str;
    fn start_generation(&self) -> u64;
    fn current_generation(&self) -> Result<u64, ()>;
    fn read_catalog(
        &mut self,
        layer: AgentAuthorityLayer,
        maximum: usize,
    ) -> Result<Option<Vec<u8>>, ()>;
    fn enforce_read_only(
        &mut self,
        request: &ReadOnlyEffectRequest,
    ) -> Result<ReadOnlyEffectEnforcement, ()>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostAgentAuthorityTransactionError {
    #[cfg(test)]
    Unsupported,
    Failed,
}
