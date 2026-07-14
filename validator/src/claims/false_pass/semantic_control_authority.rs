use super::*;

pub(crate) const MODEL_METHOD: &str = "claim-control-semantic-model-v1";
pub(crate) const MODEL_OBSERVATION_METHOD: &str = "claim-control-semantic-model-observation-v1";
pub(crate) const CONTROL_CATALOG_VERSION: &str = "adopted-claim-negative-controls-v1";
pub(crate) const MODEL_IMPLEMENTATION_VERSION: &str = "claim-control-decision-test-model-v1";
pub(crate) const MODEL_MAX_AGE_MS: u64 = 600_000;
pub(crate) const PRIVATE_TRANSPORT_UNAVAILABLE: &str =
    "claims-control-transport-unavailable:confinement-cleanup-capability-intersection-empty";
#[cfg(test)]
pub(crate) static MODEL_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Private capability for producing sealed semantic models in claim-decision
/// tests. It cannot issue or represent executed causal proof.
pub(crate) struct ModelAuthority {
    pub(crate) _private: (),
}

impl ModelAuthority {
    #[cfg(test)]
    pub(crate) fn issue_for_decision_tests() -> Self {
        Self { _private: () }
    }
}

/// Candidate-bound claim-control authority.
///
/// No adopted product executor currently satisfies the required confinement
/// and cleanup capability intersection. The production and private transports
/// therefore refuse before any scheduling, process, receipt, or filesystem
/// effect. A separate test-only semantic model exercises decision logic and is
/// rejected by the default/product ledger policy.
pub struct LocalNegativeControlAuthority<'a> {
    pub(crate) definitions: &'a ClaimDefinitions,
    pub(crate) context: LiveContext,
    pub(crate) context_id: String,
    pub(crate) candidate_id: String,
}
