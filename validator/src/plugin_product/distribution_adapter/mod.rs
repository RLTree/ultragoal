//! Package-identity-bound lifecycle effects for a descriptor-confined fixture.
//!
//! This adapter is intentionally narrower than host installation. It can only
//! target an existing [`ConfinedRoot`](crate::distribution::ConfinedRoot), and
//! it makes no package, cache, discovery, runtime, or product-journey claim.

mod binding;
mod effects;
mod error;
mod mutation;
mod observation;

use binding::BoundPackage;
use effects::Effects;
pub(crate) use error::{AdapterError, AdapterErrorId};

#[cfg(test)]
#[path = "../../../tests/plugin_distribution_adapter_contract/fixture_contract.rs"]
mod fixture_contract;
#[cfg(test)]
#[path = "../../../tests/plugin_distribution_adapter_contract/negative.rs"]
mod negative;
#[cfg(test)]
#[path = "../../../tests/plugin_distribution_adapter_contract/positive.rs"]
mod positive;
#[cfg(test)]
#[path = "../../../tests/plugin_distribution_adapter_contract/recovery/mod.rs"]
mod recovery;
#[cfg(test)]
#[path = "../../../tests/plugin_distribution_adapter_contract/security.rs"]
mod security;

use crate::distribution::{ConfinedRoot, PackagePlan, PackageSnapshot};
use crate::plugin_product::lifecycle::{
    ApplyReport, LifecycleEffect, LifecycleError, LifecyclePlan, LifecycleState, RecoveryToken,
    apply, recover, recovery_token,
};

pub(crate) struct DistributionLifecycleOperation {
    bound_plan: LifecyclePlan,
    bound_recovery_token: Option<RecoveryToken>,
    effects: Effects,
    owns_execution: bool,
}

impl DistributionLifecycleOperation {
    pub(crate) fn bind(
        root: ConfinedRoot,
        package_plan: &PackagePlan,
        package: &PackageSnapshot,
        lifecycle: &LifecyclePlan,
    ) -> Result<Self, AdapterError> {
        let bound = BoundPackage::bind(package_plan, package, lifecycle)?;
        let effects = Effects::bind(
            root,
            bound,
            lifecycle.before.clone(),
            lifecycle.expected_after.clone(),
            lifecycle.effects.clone(),
        )?;
        Ok(Self {
            bound_plan: lifecycle.clone(),
            bound_recovery_token: None,
            effects,
            owns_execution: false,
        })
    }

    pub(crate) fn apply(
        &mut self,
        observed: &LifecycleState,
        plan: &LifecyclePlan,
    ) -> Result<ApplyReport, LifecycleError> {
        if plan != &self.bound_plan {
            return Err(LifecycleError::InvalidTransition);
        }
        let result = apply(observed, plan, &mut self.effects);
        self.owns_execution |= self.effects.execution_attempted();
        result
    }

    pub(crate) fn recovery_token(&mut self) -> Result<RecoveryToken, LifecycleError> {
        if !self.owns_execution {
            return Err(LifecycleError::RecoveryUnavailable);
        }
        if self.bound_recovery_token.is_some() {
            return Err(LifecycleError::ReplayedRecoveryToken);
        }
        let token = recovery_token(&self.bound_plan)?;
        self.bound_recovery_token = Some(token.clone());
        Ok(token)
    }

    pub(crate) fn recover(
        &mut self,
        observed: &LifecycleState,
        token: &RecoveryToken,
    ) -> Result<LifecycleState, LifecycleError> {
        if !self.owns_execution || self.bound_recovery_token.as_ref() != Some(token) {
            return Err(LifecycleError::InvalidTransition);
        }
        recover(observed, token, &mut self.effects)
    }

    pub(crate) fn observe_state(&self) -> Result<LifecycleState, AdapterError> {
        self.effects.observed_state()
    }

    pub(crate) fn completed_effects(&self) -> &[LifecycleEffect] {
        self.effects.completed_effects()
    }
}
