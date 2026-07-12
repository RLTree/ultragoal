use crate::plugin_product::lifecycle::{LifecycleIntent, LifecycleState};
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HostLayer {
    Package,
    Marketplace,
    Installed,
    Cache,
    AppRegistry,
    PluginsUi,
    Discovery,
    Runtime,
}

impl HostLayer {
    pub const ALL: [Self; 8] = [
        Self::Package,
        Self::Marketplace,
        Self::Installed,
        Self::Cache,
        Self::AppRegistry,
        Self::PluginsUi,
        Self::Discovery,
        Self::Runtime,
    ];
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HostLayerVerdict {
    Verified,
    ObservedAbsent,
    Unsupported,
    CapabilityAbsent,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct HostLayerReport {
    layer: HostLayer,
    verdict: HostLayerVerdict,
    observation_sha256: Option<String>,
}

impl HostLayerReport {
    pub(crate) fn new(
        layer: HostLayer,
        verdict: HostLayerVerdict,
        observation_sha256: Option<String>,
    ) -> Self {
        Self {
            layer,
            verdict,
            observation_sha256,
        }
    }

    pub const fn layer(&self) -> HostLayer {
        self.layer
    }

    pub const fn verdict(&self) -> HostLayerVerdict {
        self.verdict
    }

    pub fn observation_sha256(&self) -> Option<&str> {
        self.observation_sha256.as_deref()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HostLifecyclePhase {
    Planned,
    AwaitingSupportedHostObservation,
    Observed,
    TeardownObserved,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PluginsUiObservation {
    verdict: HostLayerVerdict,
    observation_sha256: Option<String>,
    binding_sha256: String,
}

impl PluginsUiObservation {
    pub(crate) fn new(
        verdict: HostLayerVerdict,
        observation_sha256: Option<String>,
        binding_sha256: String,
    ) -> Self {
        Self {
            verdict,
            observation_sha256,
            binding_sha256,
        }
    }

    pub const fn verdict(&self) -> HostLayerVerdict {
        self.verdict
    }

    pub fn observation_sha256(&self) -> Option<&str> {
        self.observation_sha256.as_deref()
    }

    pub fn binding_sha256(&self) -> &str {
        &self.binding_sha256
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct HostLifecycleReport {
    intent: LifecycleIntent,
    phase: HostLifecyclePhase,
    state: LifecycleState,
    layers: Vec<HostLayerReport>,
    external_effect_request_sha256: Option<String>,
    external_effect_consumed: bool,
    identity_chain_sha256: Option<String>,
    claim_effect: bool,
}

impl HostLifecycleReport {
    pub(crate) fn new(
        intent: LifecycleIntent,
        phase: HostLifecyclePhase,
        state: LifecycleState,
        layers: Vec<HostLayerReport>,
        external_effect_request_sha256: Option<String>,
        external_effect_consumed: bool,
        identity_chain_sha256: Option<String>,
    ) -> Self {
        Self {
            intent,
            phase,
            state,
            layers,
            external_effect_request_sha256,
            external_effect_consumed,
            identity_chain_sha256,
            claim_effect: false,
        }
    }

    pub const fn intent(&self) -> LifecycleIntent {
        self.intent
    }

    pub const fn phase(&self) -> HostLifecyclePhase {
        self.phase
    }

    pub fn state(&self) -> &LifecycleState {
        &self.state
    }

    pub fn layers(&self) -> &[HostLayerReport] {
        &self.layers
    }

    pub fn layer(&self, layer: HostLayer) -> &HostLayerReport {
        self.layers
            .iter()
            .find(|row| row.layer == layer)
            .expect("complete host layer set")
    }

    pub fn external_effect_request_sha256(&self) -> Option<&str> {
        self.external_effect_request_sha256.as_deref()
    }

    pub const fn external_effect_consumed(&self) -> bool {
        self.external_effect_consumed
    }

    pub fn identity_chain_sha256(&self) -> Option<&str> {
        self.identity_chain_sha256.as_deref()
    }

    pub const fn has_claim_effect(&self) -> bool {
        self.claim_effect
    }
}
