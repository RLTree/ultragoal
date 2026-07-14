use super::capability_gate::HostEffectCapabilityGate;
use super::effect_request::{
    ExternalHostEffectRequest, HostScopeAuthority, PreparedExternalHostEffect,
};
use super::error::{HostLifecycleError, HostLifecycleErrorId};
use super::issuance::SessionIssuance;
use super::model::{
    HostLayer, HostLayerReport, HostLayerVerdict, HostLifecyclePhase, HostLifecycleReport,
};
use super::observation::{
    BoundHostObservationTransaction, HostObservationFrame, HostObservationTransactionRequest,
    HostSurfaceReader, HostSurfaceTransaction, HostSurfaceTransactionError,
};
use super::scope::BoundHostScope;
use super::verify::verify_host_identity_chain;
use crate::distribution::{
    ConfinedRoot, HostCapabilityDeclaration, JourneyBinding, MarketplacePlan, PackagePlan,
    PackageSnapshot,
};
use crate::plugin_product::distribution_adapter::DistributionLifecycleOperation;
use crate::plugin_product::lifecycle::{
    ApplyDisposition, ApplyReport, LifecycleIntent, LifecyclePlan, LifecycleState,
};
use sha2::{Digest, Sha256};
use std::sync::Arc;

include!("host_lifecycle/session.rs");

include!("host_lifecycle/binding.rs");

include!("host_lifecycle/observation_capture.rs");

include!("ordered_layers.rs");

#[cfg(test)]
mod tests;
