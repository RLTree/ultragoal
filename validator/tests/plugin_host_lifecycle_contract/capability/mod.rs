use crate::distribution::{
    Capability, HostCapabilityDeclaration, HostCapabilityState, MarketplaceScope,
};
use crate::host_fixture::{Fixture, installed, lifecycle, marketplace_plan_named, request};
use crate::host_lifecycle::{
    HostLifecycleErrorId, HostScopeAuthority, capability_states_supported,
    required_host_capabilities,
};
use crate::plugin_product::lifecycle::{LifecycleIntent, LifecycleState};

include!("capability/effectful.rs");

include!("capability/no_effect_intents_do_not_promote_or_require_external_host_capability.rs");

include!("capability/unavailable_session.rs");
