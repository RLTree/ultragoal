mod transitions;

use super::model::{
    LifecycleEffect, LifecycleError, LifecycleIntent, LifecyclePlan, LifecycleRequest,
    LifecycleState, PlanAuthorizationSeal, validate_digest,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

pub fn plan(
    observed: &LifecycleState,
    request: &LifecycleRequest,
) -> Result<LifecyclePlan, LifecycleError> {
    let canonical = derive_canonical_plan(observed, request)?;
    Ok(LifecyclePlan {
        schema_version: PLAN_SCHEMA_VERSION.to_owned(),
        plan_id: canonical.plan_id,
        authorization_sha256: canonical.authorization_sha256,
        intent: request.intent,
        before: observed.clone(),
        expected_after: canonical.expected_after,
        effects: canonical.effects,
        rollback_state: observed.clone(),
        writes_host_state: canonical.writes_host_state,
        authorization_seal: PlanAuthorizationSeal::issue(observed, request),
    })
}

const PLAN_SCHEMA_VERSION: &str = "HarnessPluginLifecyclePlan-v1";

struct CanonicalPlan {
    plan_id: String,
    authorization_sha256: String,
    expected_after: LifecycleState,
    effects: Vec<LifecycleEffect>,
    writes_host_state: bool,
}

fn derive_canonical_plan(
    observed: &LifecycleState,
    request: &LifecycleRequest,
) -> Result<CanonicalPlan, LifecycleError> {
    observed.validate()?;
    request.authorization.validate()?;
    if let Some(target) = &request.target {
        target.validate()?;
    }
    if let Some(prior) = &request.prior_authority {
        prior.validate()?;
    }
    validate_request_shape(request)?;
    transitions::enforce_expected_prior(observed, &request.authorization)?;
    let (mut expected_after, effects) = transitions::transition(observed, request)?;
    let writes_host_state = transitions::writes_host_state(&effects);
    expected_after.validate()?;
    if !writes_host_state {
        expected_after.generation = observed.generation;
    }
    let authorization_sha256 = digest_value(&request.authorization)?;
    let plan_id = plan_digest(
        request.intent,
        observed,
        &expected_after,
        &effects,
        writes_host_state,
        &authorization_sha256,
    )?;
    Ok(CanonicalPlan {
        plan_id,
        authorization_sha256,
        expected_after,
        effects,
        writes_host_state,
    })
}

fn validate_request_shape(request: &LifecycleRequest) -> Result<(), LifecycleError> {
    let valid = match request.intent {
        LifecycleIntent::FreshInstall
        | LifecycleIntent::MonotonicUpdate
        | LifecycleIntent::AuthorizedRollback
        | LifecycleIntent::IdempotentReinstall => {
            request.target.is_some() && request.prior_authority.is_none()
        }
        LifecycleIntent::FailedUpdateRecovery => {
            request.target.is_none() && request.prior_authority.is_some()
        }
        LifecycleIntent::UninstallTeardown | LifecycleIntent::StaleCacheRecovery => {
            request.target.is_none() && request.prior_authority.is_none()
        }
        LifecycleIntent::RepeatUse => request.prior_authority.is_none(),
    };
    valid.then_some(()).ok_or(LifecycleError::InvalidTransition)
}

pub(super) fn validate_plan(plan: &LifecyclePlan) -> Result<(), LifecycleError> {
    if plan.schema_version != PLAN_SCHEMA_VERSION || plan.effects.is_empty() {
        return Err(LifecycleError::InvalidTransition);
    }
    plan.before.validate()?;
    plan.expected_after.validate()?;
    plan.rollback_state.validate()?;
    validate_digest(&plan.authorization_sha256)?;
    validate_digest(&plan.plan_id)?;
    if transitions::writes_host_state(&plan.effects) != plan.writes_host_state {
        return Err(LifecycleError::InvalidTransition);
    }
    let expected = plan_digest(
        plan.intent,
        &plan.before,
        &plan.expected_after,
        &plan.effects,
        plan.writes_host_state,
        &plan.authorization_sha256,
    )?;
    if expected != plan.plan_id || plan.rollback_state != plan.before {
        return Err(LifecycleError::InvalidTransition);
    }
    let Some((authorized_observed, authorized_request)) =
        plan.authorization_seal.authorized_context()
    else {
        return Err(LifecycleError::UnsealedPlan);
    };
    let canonical = derive_canonical_plan(authorized_observed, authorized_request)?;
    if plan.intent != authorized_request.intent
        || plan.before != *authorized_observed
        || plan.rollback_state != *authorized_observed
        || plan.plan_id != canonical.plan_id
        || plan.authorization_sha256 != canonical.authorization_sha256
        || plan.expected_after != canonical.expected_after
        || plan.effects != canonical.effects
        || plan.writes_host_state != canonical.writes_host_state
    {
        return Err(LifecycleError::InvalidTransition);
    }
    Ok(())
}

pub(super) fn consume_plan(plan: &LifecyclePlan) -> Result<(), LifecycleError> {
    plan.authorization_seal.consume()
}

pub(super) fn plan_digest(
    intent: LifecycleIntent,
    before: &LifecycleState,
    after: &LifecycleState,
    effects: &[LifecycleEffect],
    writes_host_state: bool,
    authorization_sha256: &str,
) -> Result<String, LifecycleError> {
    let bytes = serde_json::to_vec(&(
        intent,
        before,
        after,
        effects,
        writes_host_state,
        authorization_sha256,
    ))
    .map_err(|_| LifecycleError::InvalidTransition)?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
pub(super) fn record_writes_host_state(effects: &[LifecycleEffect]) -> bool {
    transitions::writes_host_state(effects)
}

fn digest_value<T: Serialize>(value: &T) -> Result<String, LifecycleError> {
    let bytes = serde_json::to_vec(value).map_err(|_| LifecycleError::InvalidTransition)?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}
