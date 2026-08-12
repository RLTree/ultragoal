use super::StateEngine;
use super::adopted_claims::{RootClaimStage, stage_current_authority};
use super::catalog::{
    ClaimSpec, DependencyActionCatalog, DependencyActionSpec, HostGoalObservation, RuntimeMetadata,
};
use super::policy_authority::PolicyAuthority;
use super::product_state::StateError;
use super::snapshot::{CurrentAuthorityCatalog, StateAuthorityCatalog};
use crate::context::LiveContext;
use crate::inventory::AuthorityCatalog;

const CLAIM_ID: &str = "CL-USABLE-LOOP";
const CLAIM_DIMENSION: &str = "tested-journey";

/// Derives current product state from the sole live current-authority catalog.
/// This route does not construct or consume a retained inventory catalog.
pub(crate) fn derive_current(context: &LiveContext) -> Result<super::ProductState, StateError> {
    let authority_catalog = CurrentAuthorityCatalog::capture(context)?;
    let policy = issue_current(context, &authority_catalog)?;
    StateEngine::derive_with_catalog(context, &authority_catalog, &policy)
}

fn issue_current(
    context: &LiveContext,
    authority_catalog: &CurrentAuthorityCatalog,
) -> Result<DependencyActionCatalog, StateError> {
    let authority_digest = crate::product_inception::current_authority_digest(context)
        .map_err(|_| StateError::InvalidCatalog("current-product-authority-invalid".to_owned()))?;
    let claims = current_claims();
    let spec = live_spec(context, authority_catalog, &claims);
    PolicyAuthority::from_current_authority(authority_digest, claims, spec)?
        .issue(context, authority_catalog)
}

/// Frozen compatibility-only state derivation kept for retained fixture tests.
/// Production current routes must call `derive_current` above.
#[cfg(test)]
pub(crate) fn derive_adopted(
    context: &LiveContext,
    authority_catalog: &AuthorityCatalog,
) -> Result<super::ProductState, StateError> {
    let policy = issue_adopted(context, authority_catalog)?;
    StateEngine::derive(context, authority_catalog, &policy)
}

/// Issues the sole current product claim from the current goal, product
/// contract, and active ExecPlan.  The retained v2 registry is not read here.
#[cfg(test)]
pub(crate) fn issue_adopted(
    context: &LiveContext,
    authority_catalog: &AuthorityCatalog,
) -> Result<DependencyActionCatalog, StateError> {
    let authority_digest = crate::product_inception::current_authority_digest(context)
        .map_err(|_| StateError::InvalidCatalog("current-product-authority-invalid".to_owned()))?;
    let claims = current_claims();
    let spec = live_spec(context, authority_catalog, &claims);
    PolicyAuthority::from_current_authority(authority_digest, claims, spec)?
        .issue(context, authority_catalog)
}

/// The strict compatibility adapter receives a withheld stage. It cannot
/// project a current product claim and has no route through the frozen v2
/// claim registry.
pub(crate) fn stage_root_claims(
    context: &LiveContext,
    authority_catalog: &AuthorityCatalog,
) -> Result<RootClaimStage, StateError> {
    let authority_digest = crate::product_inception::current_authority_digest(context)
        .map_err(|_| StateError::InvalidCatalog("current-product-authority-invalid".to_owned()))?;
    stage_current_authority(context, authority_catalog, authority_digest)
}

fn current_claims() -> Vec<ClaimSpec> {
    vec![ClaimSpec {
        claim_id: CLAIM_ID.to_owned(),
        maximum_dimensions: vec![CLAIM_DIMENSION.to_owned()],
    }]
}

fn live_spec<C: StateAuthorityCatalog + ?Sized>(
    context: &LiveContext,
    authority_catalog: &C,
    claims: &[ClaimSpec],
) -> DependencyActionSpec {
    let reductions = super::CeilingReduction {
        claim_id: CLAIM_ID.to_owned(),
        dimensions: std::collections::BTreeSet::from([CLAIM_DIMENSION.to_owned()]),
    };
    let mut dependencies = Vec::new();
    let mut commands = vec![super::CommandBinding {
        command_id: "inspect-json".to_owned(),
        argv: vec![
            "ultragoal".to_owned(),
            "--json".to_owned(),
            "inspect".to_owned(),
        ],
        effect: crate::context::EffectClass::Read,
    }];
    let mut actions = Vec::new();
    super::adopted_journey::append_current_usable_loop_route(
        &mut dependencies,
        &mut commands,
        &mut actions,
        &[reductions],
    );
    DependencyActionSpec {
        expected_context_id: context.context_id().to_owned(),
        expected_authority_catalog_id: authority_catalog.catalog_id().to_owned(),
        claims: claims.to_vec(),
        dependencies,
        // Current authority binds the catalog identity but intentionally does
        // not turn retained compatibility findings into claim impacts/actions.
        inventory_policies: Vec::new(),
        capability_requirements: Vec::new(),
        runtime_metadata: RuntimeMetadata::default(),
        runtime_requirements: Vec::new(),
        commands,
        actions,
        host_goal: HostGoalObservation::default(),
    }
}
