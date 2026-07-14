use super::capability_gate::{HostEffectCapabilityGate, same_gate};
use super::error::{HostLifecycleError, HostLifecycleErrorId};
use super::issuance::{SessionIssuance, same_issuance};
use super::scope::{BoundHostScope, same_scope};
use crate::distribution::{HostCommandPlan, JourneyBinding, MarketplaceScope, PackageIdentity};
use crate::plugin_product::lifecycle::{LifecycleIntent, LifecyclePlan};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;

include!("host_scope_authority.rs");

include!("external_host_effect_request_for_intent.rs");

include!("prepared_external_host_effect_intent.rs");
