#[cfg(test)]
use super::host_custody::recovery_state_after_completed_prefix;
#[cfg(test)]
use super::model::{ApplyDisposition, ApplyReport};
use super::model::{
    LifecycleEffectAdapter, LifecycleError, LifecyclePlan, LifecycleState,
    RecoveryAuthorizationSeal, RecoveryToken, validate_digest,
};
#[cfg(test)]
use super::plan::consume_plan;
use super::plan::validate_plan;

#[cfg(test)]
pub(crate) fn apply<A: LifecycleEffectAdapter>(
    observed: &LifecycleState,
    plan: &LifecyclePlan,
    adapter: &mut A,
) -> Result<ApplyReport, LifecycleError> {
    validate_plan(plan)?;
    if observed != &plan.before {
        return Err(LifecycleError::StalePlan);
    }
    consume_plan(plan)?;
    let mut completed_effects = Vec::new();
    for effect in &plan.effects {
        if let Err(causal_error) = adapter.execute(*effect, &plan.expected_after) {
            if !plan.writes_host_state {
                plan.authorization_seal.finish_apply(None)?;
                return Err(LifecycleError::ReadEffectFailed {
                    effect: *effect,
                    causal_error,
                });
            }
            if let Err(recovery_error) = adapter.restore(&plan.rollback_state) {
                let expected_recovery_state =
                    recovery_state_after_completed_prefix(plan, &completed_effects)?;
                let observed_recovery_state = match adapter.observe_state() {
                    Ok(state) => state,
                    Err(error) => {
                        plan.authorization_seal.finish_apply(None)?;
                        return Err(LifecycleError::RecoveryObservationFailed(error));
                    }
                };
                if observed_recovery_state.validate().is_err()
                    || observed_recovery_state != expected_recovery_state
                {
                    plan.authorization_seal.finish_apply(None)?;
                    return Err(LifecycleError::RecoveryStateMismatch);
                }
                plan.authorization_seal
                    .finish_apply(Some(&observed_recovery_state))?;
                return Err(LifecycleError::RecoveryFailed(recovery_error));
            }
            plan.authorization_seal.finish_apply(None)?;
            return Ok(ApplyReport {
                plan_id: plan.plan_id.clone(),
                disposition: ApplyDisposition::RecoveredAfterFailure,
                state: plan.rollback_state.clone(),
                completed_effects,
                failed_effect: Some(*effect),
                causal_error: Some(causal_error),
            });
        }
        completed_effects.push(*effect);
    }
    let recovery_state = plan.writes_host_state.then_some(&plan.expected_after);
    plan.authorization_seal.finish_apply(recovery_state)?;
    Ok(ApplyReport {
        plan_id: plan.plan_id.clone(),
        disposition: ApplyDisposition::Applied,
        state: plan.expected_after.clone(),
        completed_effects,
        failed_effect: None,
        causal_error: None,
    })
}

pub(crate) fn recovery_token(plan: &LifecyclePlan) -> Result<RecoveryToken, LifecycleError> {
    validate_plan(plan)?;
    if !plan.writes_host_state {
        return Err(LifecycleError::InvalidTransition);
    }
    let (action_state, recovery_state) = plan.authorization_seal.recovery_authority()?;
    Ok(RecoveryToken {
        schema_version: "HarnessPluginLifecycleRecoveryToken-v1".to_owned(),
        plan_id: plan.plan_id.clone(),
        prior: plan.rollback_state.clone(),
        expected_current: plan.expected_after.clone(),
        authorization_seal: RecoveryAuthorizationSeal::issue(
            &plan.plan_id,
            &plan.rollback_state,
            &plan.expected_after,
            action_state,
            recovery_state,
        ),
    })
}

pub(crate) fn recover<A: LifecycleEffectAdapter>(
    observed: &LifecycleState,
    token: &RecoveryToken,
    adapter: &mut A,
) -> Result<LifecycleState, LifecycleError> {
    validate_recovery_token(token)?;
    observed.validate()?;
    token.authorization_seal.consume_for_observed(observed)?;
    let result = adapter.restore(&token.prior);
    token.authorization_seal.close()?;
    result.map_err(LifecycleError::RecoveryFailed)?;
    Ok(token.prior.clone())
}

fn validate_recovery_token(token: &RecoveryToken) -> Result<(), LifecycleError> {
    if token.schema_version != "HarnessPluginLifecycleRecoveryToken-v1" {
        return Err(LifecycleError::InvalidTransition);
    }
    validate_digest(&token.plan_id)?;
    token.prior.validate()?;
    token.expected_current.validate()?;
    let Some((plan_id, prior, expected_current)) = token.authorization_seal.authorized_restore()
    else {
        return Err(LifecycleError::UnsealedRecoveryToken);
    };
    if token.plan_id != plan_id
        || token.prior != *prior
        || token.expected_current != *expected_current
    {
        return Err(LifecycleError::InvalidTransition);
    }
    Ok(())
}
