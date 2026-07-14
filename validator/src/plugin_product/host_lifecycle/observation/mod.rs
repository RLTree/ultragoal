use super::error::{HostLifecycleError, HostLifecycleErrorId};
use super::issuance::{SessionIssuance, same_issuance};
use super::model::{HostLayer, HostLayerReport, HostLayerVerdict, PluginsUiObservation};
use super::scope::{BoundHostScope, same_scope};
use crate::distribution::{
    AppRegistryObservation, AppRegistryVerdict, CacheSnapshot, Capability, DiscoveryObservation,
    DiscoveryVerdict, HostCapabilityState, JourneyBinding, MarketplacePlan, MarketplaceSnapshot,
    RuntimeObservation, observe_app_registry, observe_codex_marketplace, observe_discovery,
    reconcile_cache_read_only,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt;
use std::sync::Arc;

include!("observation_limit.rs");

include!("host_observation_frame_binding_sha256.rs");

include!("validate_frame.rs");

include!("plugins_ui.rs");
