//! Product-shaped coordination across the distinct plugin distribution layers.
//!
//! This module owns no manifest, package, host, command, or claim authority. It
//! composes already-issued package and lifecycle authority, confines local
//! effects through the accepted distribution adapter, and validates supplied
//! observations without executing a live-host command.

mod capability_gate;
mod darwin;
mod effect_request;
mod error;
mod issuance;
mod model;
mod observation;
mod scope;
mod session;
mod verify;

#[cfg(test)]
pub(crate) use capability_gate::{capability_states_supported, required_host_capabilities};
#[cfg(test)]
pub use darwin::{
    DarwinHostDiagnosis, DarwinHostError, DarwinHostErrorId, DarwinHostOperation,
    DarwinHostSnapshot, DarwinHostSurface, DarwinHostTransactionAdapter,
    DarwinHostTransactionDisposition, DarwinHostTransactionPlan, DarwinHostTransactionReport,
    DarwinSurfaceObservation, DarwinSurfaceStatus,
};
#[cfg(test)]
pub use darwin::{DarwinTestControl, DarwinTestPoint};
#[cfg(test)]
pub use effect_request::{
    ExternalHostEffectRequest, HostScopeAuthority, PreparedExternalHostEffect,
};
#[cfg(test)]
pub use error::{HostLifecycleError, HostLifecycleErrorId};
#[cfg(test)]
pub use model::{
    HostLayer, HostLayerReport, HostLayerVerdict, HostLifecyclePhase, HostLifecycleReport,
    PluginsUiObservation,
};
#[cfg(test)]
pub use observation::{
    HostObservationFrame, HostObservationTransactionRequest, HostSurfaceReader,
    HostSurfaceTransaction, HostSurfaceTransactionError, observe_plugins_ui,
};
#[cfg(test)]
pub use session::{HostLifecycleBindRequest, HostLifecycleSession};
#[cfg(test)]
pub use verify::verify_host_identity_chain;
