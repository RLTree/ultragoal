use super::error::{AgentDiscoveryError, AgentDiscoveryErrorId};
use super::host::{
    BoundHostAgentAuthorityTransaction, HostAgentAuthorityReader, HostAgentAuthorityRequest,
    HostAgentAuthorityTransactionError, parse_and_verify_capture,
};
use super::model::AgentRouteEligibility;
use super::protocol_codec::{AgentProtocolCodecRequest, VerifiedLayerBinding, encode_digest};
use super::source::SourceAgentCatalog;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};

const BOUND: u8 = 0;
const VERIFYING: u8 = 1;
const CLOSED: u8 = 2;
const REJECTED: u8 = 3;
static NEXT_ISSUANCE: AtomicU64 = AtomicU64::new(1);

struct SessionIssuance {
    issuance_sha256: String,
    observation_nonce_sha256: String,
    state: AtomicU8,
}

pub struct AgentDiscoverySession {
    source: SourceAgentCatalog,
    binding_sha256: String,
    request: HostAgentAuthorityRequest,
    issuance: Arc<SessionIssuance>,
}

impl AgentDiscoverySession {
    pub fn bind(source: SourceAgentCatalog) -> Result<Self, AgentDiscoveryError> {
        source.revalidate()?;
        let unique = NEXT_ISSUANCE.fetch_add(1, Ordering::Relaxed);
        let issuance_sha256 = encode_digest(AgentProtocolCodecRequest::SessionIssuance {
            project_root_sha256: source.project_root_sha256(),
            candidate_id: source.candidate_id(),
            session_id: source.session_id(),
            catalog_sha256: source.catalog_sha256(),
            process_id: std::process::id(),
            unique,
        })
        .map(|response| response.sha256())
        .map_err(|_| invalid())?;
        let observation_nonce_sha256 = encode_digest(AgentProtocolCodecRequest::ObservationNonce {
            issuance_sha256: &issuance_sha256,
            unique,
        })
        .map(|response| response.sha256())
        .map_err(|_| invalid())?;
        let binding_sha256 = encode_digest(AgentProtocolCodecRequest::SessionBinding {
            project_root_sha256: source.project_root_sha256(),
            candidate_id: source.candidate_id(),
            session_id: source.session_id(),
            catalog_sha256: source.catalog_sha256(),
            plugin_version: source.plugin_version(),
            plugin_manifest_sha256: source.plugin_manifest_sha256(),
            issuance_sha256: &issuance_sha256,
            observation_nonce_sha256: &observation_nonce_sha256,
        })
        .map(|response| response.sha256())
        .map_err(|_| invalid())?;
        let provenance_sha256 = encode_digest(AgentProtocolCodecRequest::TransactionProvenance {
            binding_sha256: &binding_sha256,
            project_root_sha256: source.project_root_sha256(),
            candidate_id: source.candidate_id(),
            session_id: source.session_id(),
            issuance_sha256: &issuance_sha256,
            observation_nonce_sha256: &observation_nonce_sha256,
        })
        .map(|response| response.sha256())
        .map_err(|_| invalid())?;
        let request = HostAgentAuthorityRequest::issue(
            provenance_sha256,
            source.project_root_sha256().to_owned(),
            source.candidate_id().to_owned(),
            source.session_id().to_owned(),
            issuance_sha256.clone(),
            observation_nonce_sha256.clone(),
        );
        Ok(Self {
            source,
            binding_sha256,
            request,
            issuance: Arc::new(SessionIssuance {
                issuance_sha256,
                observation_nonce_sha256,
                state: AtomicU8::new(BOUND),
            }),
        })
    }

    #[cfg(test)]
    pub fn binding_sha256(&self) -> &str {
        &self.binding_sha256
    }

    pub fn verify(
        &self,
        reader: &mut impl HostAgentAuthorityReader,
    ) -> Result<AgentRouteEligibility, AgentDiscoveryError> {
        self.begin()?;
        let result = self.verify_inner(reader);
        if result.is_err() {
            self.issuance.state.store(REJECTED, Ordering::Release);
        }
        result
    }

    fn verify_inner(
        &self,
        reader: &mut impl HostAgentAuthorityReader,
    ) -> Result<AgentRouteEligibility, AgentDiscoveryError> {
        self.source.revalidate()?;
        reader.with_transaction(&self.request, |transaction| {
            let transaction = transaction.map_err(transaction_error)?;
            let mut transaction =
                BoundHostAgentAuthorityTransaction::bind(transaction, &self.request)?;
            self.source.revalidate()?;
            let first = transaction.capture_raw(&self.request)?;
            self.source.revalidate()?;
            let second = transaction.capture_raw(&self.request)?;
            if first != second {
                return Err(AgentDiscoveryError::new(
                    AgentDiscoveryErrorId::ObservationChanged,
                ));
            }
            transaction.require_current(&self.request)?;
            let layers = parse_and_verify_capture(&first, &self.source, &self.request)?;
            let host_binding_rows = layers
                .iter()
                .map(|layer| VerifiedLayerBinding {
                    layer: layer.layer(),
                    catalog_sha256: layer.catalog_sha256(),
                    authority_root_sha256: layer.authority_root_sha256(),
                    authority_generation_sha256: layer.authority_generation_sha256(),
                    transaction_provenance_sha256: layer.transaction_provenance_sha256(),
                })
                .collect::<Vec<_>>();
            let verified_binding_sha256 =
                encode_digest(AgentProtocolCodecRequest::VerifiedBinding {
                    binding_sha256: &self.binding_sha256,
                    rows: &host_binding_rows,
                })
                .map(|response| response.sha256())
                .map_err(|_| invalid())?;
            self.source.revalidate()?;
            let sandbox_effect_sha256 = transaction.enforce_effects(
                &self.source,
                &verified_binding_sha256,
                &self.request,
            )?;
            self.source.revalidate()?;
            transaction.require_current(&self.request)?;
            if self.issuance.issuance_sha256 != self.request.session_issuance_sha256()
                || self.issuance.observation_nonce_sha256 != self.request.observation_nonce_sha256()
            {
                return Err(invalid());
            }
            self.issuance
                .state
                .compare_exchange(VERIFYING, CLOSED, Ordering::AcqRel, Ordering::Acquire)
                .map_err(|_| {
                    AgentDiscoveryError::new(AgentDiscoveryErrorId::SessionStateRejected)
                })?;
            self.source.revalidate()?;
            transaction.require_current(&self.request)?;
            let new_session_observed = layers
                .first()
                .is_some_and(|layer| layer.new_session_observed());
            Ok(AgentRouteEligibility::observed(
                verified_binding_sha256,
                self.source.catalog_sha256().to_owned(),
                self.source.plugin_version().to_owned(),
                layers,
                sandbox_effect_sha256,
                new_session_observed,
            ))
        })
    }

    fn begin(&self) -> Result<(), AgentDiscoveryError> {
        self.issuance
            .state
            .compare_exchange(BOUND, VERIFYING, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| ())
            .map_err(|state| {
                AgentDiscoveryError::new(if state == CLOSED || state == VERIFYING {
                    AgentDiscoveryErrorId::SessionReplay
                } else {
                    AgentDiscoveryErrorId::SessionStateRejected
                })
            })
    }
}

fn transaction_error(error: HostAgentAuthorityTransactionError) -> AgentDiscoveryError {
    match error {
        #[cfg(test)]
        HostAgentAuthorityTransactionError::Unsupported => {
            AgentDiscoveryError::new(AgentDiscoveryErrorId::ObservationUnavailable)
        }
        HostAgentAuthorityTransactionError::Failed => {
            AgentDiscoveryError::new(AgentDiscoveryErrorId::ObservationUnavailable)
        }
    }
}

fn invalid() -> AgentDiscoveryError {
    AgentDiscoveryError::new(AgentDiscoveryErrorId::InvalidBinding)
}
