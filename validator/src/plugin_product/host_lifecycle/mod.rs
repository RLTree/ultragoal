//! Product-shaped coordination across the distinct plugin distribution layers.
//!
//! This module owns no manifest, package, host, command, or claim authority. It
//! composes already-issued package and lifecycle authority, confines local
//! effects through the accepted distribution adapter, and validates supplied
//! observations without executing a live-host command.

mod capability_gate;
mod effect_request;
mod error;
mod issuance;
mod model;
mod observation;
mod scope;
mod session;
mod verify;

pub(crate) use capability_gate::{capability_states_supported, required_host_capabilities};
pub use effect_request::{
    ExternalHostEffectRequest, HostScopeAuthority, PreparedExternalHostEffect,
};
pub use error::{HostLifecycleError, HostLifecycleErrorId};
pub use model::{
    HostLayer, HostLayerReport, HostLayerVerdict, HostLifecyclePhase, HostLifecycleReport,
    PluginsUiObservation,
};
pub use observation::{
    HostObservationFrame, HostObservationTransactionRequest, HostSurfaceReader,
    HostSurfaceTransaction, HostSurfaceTransactionError, observe_plugins_ui,
};
pub use session::HostLifecycleSession;
pub use verify::verify_host_identity_chain;
