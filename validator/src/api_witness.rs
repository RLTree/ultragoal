use crate::context::{
    BuildRequest, CandidateIdentity, CapabilitySet, ContextError, EffectClass, LiveContext,
};
use crate::inventory::{AuthorityCatalog, GeneratedSurfaceIndex, InventoryBuilder};

fn require_type<T>() {}

pub(crate) fn implemented_public_apis() -> &'static [&'static str] {
    let _: fn(BuildRequest) -> Result<LiveContext, ContextError> = LiveContext::build;
    require_type::<EffectClass>();
    require_type::<CapabilitySet>();
    require_type::<CandidateIdentity>();
    require_type::<InventoryBuilder<'static>>();
    require_type::<AuthorityCatalog>();
    require_type::<GeneratedSurfaceIndex>();
    &[
        "LiveContext::build",
        "EffectClass",
        "CapabilitySet",
        "CandidateIdentity",
        "InventoryBuilder",
        "AuthorityCatalog",
        "GeneratedSurfaceIndex",
    ]
}
