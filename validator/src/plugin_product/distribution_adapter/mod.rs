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
pub use error::{AdapterError, AdapterErrorId};

use crate::distribution::{ConfinedRoot, PackagePlan, PackageSnapshot};
use crate::plugin_product::lifecycle::{
    ApplyReport, LifecycleEffect, LifecycleError, LifecyclePlan, LifecycleState, RecoveryToken,
    apply, recover, recovery_token,
};

pub struct DistributionLifecycleOperation {
    bound_plan: LifecyclePlan,
    bound_recovery_token: Option<RecoveryToken>,
    effects: Effects,
    owns_execution: bool,
}

impl DistributionLifecycleOperation {
    pub fn bind(
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

    pub fn apply(
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

    pub fn recovery_token(&mut self) -> Result<RecoveryToken, LifecycleError> {
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

    pub fn recover(
        &mut self,
        observed: &LifecycleState,
        token: &RecoveryToken,
    ) -> Result<LifecycleState, LifecycleError> {
        if !self.owns_execution || self.bound_recovery_token.as_ref() != Some(token) {
            return Err(LifecycleError::InvalidTransition);
        }
        recover(observed, token, &mut self.effects)
    }

    pub fn observe_state(&self) -> Result<LifecycleState, AdapterError> {
        self.effects.observed_state()
    }

    pub fn completed_effects(&self) -> &[LifecycleEffect] {
        self.effects.completed_effects()
    }

    pub fn observed_mutation_count(&self) -> usize {
        self.effects.mutation_count()
    }
}
