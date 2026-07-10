use ultragoal::context::{
    BuildRequest, CandidateIdentity, CapabilitySet, ContextError, EffectClass, LiveContext,
};
use ultragoal::inventory::{AuthorityCatalog, GeneratedSurfaceIndex, InventoryBuilder};

fn require_public_type<T>() {}

#[test]
fn contract_public_api_witness_compiles_outside_the_library_crate() {
    let _: fn(BuildRequest) -> Result<LiveContext, ContextError> = LiveContext::build;
    require_public_type::<EffectClass>();
    require_public_type::<CapabilitySet>();
    require_public_type::<CandidateIdentity>();
    require_public_type::<InventoryBuilder<'static>>();
    require_public_type::<AuthorityCatalog>();
    require_public_type::<GeneratedSurfaceIndex>();
}
