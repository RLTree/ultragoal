use super::transaction_observation::HostLifecycleObservationResult;
use crate::plugin_product::lifecycle::{LifecycleEffect, LifecyclePlan, LifecycleState};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SurfacePresence {
    Present,
    Absent,
}

impl SurfacePresence {
    pub(super) fn from_path(present: bool) -> Self {
        if present { Self::Present } else { Self::Absent }
    }

    pub(super) fn from_json(identity: Option<&String>) -> Self {
        if identity.is_some() {
            Self::Present
        } else {
            Self::Absent
        }
    }
}

struct ReadOnlyLifecycleAdapter {
    completed: Vec<LifecycleEffect>,
    observed: LifecycleState,
}

impl crate::plugin_product::lifecycle::model::LifecycleEffectAdapter for ReadOnlyLifecycleAdapter {
    fn execute(
        &mut self,
        effect: LifecycleEffect,
        _expected_after: &LifecycleState,
    ) -> Result<(), String> {
        if self.completed.first().copied() == Some(effect) {
            self.completed.remove(0);
            Ok(())
        } else {
            Err("independent lifecycle observation prefix mismatch".to_owned())
        }
    }

    fn restore(&mut self, _prior: &LifecycleState) -> Result<(), String> {
        Err("read-only lifecycle route cannot restore host state".to_owned())
    }

    fn observe_state(&self) -> Result<LifecycleState, String> {
        Ok(self.observed.clone())
    }
}

pub(crate) fn validate_read_only_transition(
    plan: &LifecyclePlan,
    result: &HostLifecycleObservationResult,
) -> Result<(), &'static str> {
    let mut adapter = ReadOnlyLifecycleAdapter {
        completed: result.completed_effects.clone(),
        observed: result.observed.clone(),
    };
    crate::plugin_product::lifecycle::execution::apply(&plan.before, plan, &mut adapter)
        .map(|_| ())
        .map_err(|_| "read-only lifecycle transition validation failed")
}

pub(super) fn derive_effect_prefix(
    plan: &LifecyclePlan,
    installed: SurfacePresence,
    cache: SurfacePresence,
    runtime: SurfacePresence,
    registry: SurfacePresence,
    discovery: SurfacePresence,
) -> Result<Vec<LifecycleEffect>, &'static str> {
    let mut completed = Vec::new();
    for effect in &plan.effects {
        let valid = match effect {
            LifecycleEffect::InstallPackage | LifecycleEffect::VerifyInstalledBytes => {
                installed == SurfacePresence::Present
            }
            LifecycleEffect::RefreshCache => cache == SurfacePresence::Present,
            LifecycleEffect::RestorePriorAuthority => {
                installed == SurfacePresence::Present && cache == SurfacePresence::Present
            }
            LifecycleEffect::RemoveInstalledPackage => installed == SurfacePresence::Absent,
            LifecycleEffect::RemoveCache => cache == SurfacePresence::Absent,
            LifecycleEffect::VerifyTeardown => {
                installed == SurfacePresence::Absent
                    && cache == SurfacePresence::Absent
                    && registry == SurfacePresence::Absent
                    && discovery == SurfacePresence::Absent
                    && runtime == SurfacePresence::Absent
            }
            LifecycleEffect::ProbeRuntime => {
                runtime == SurfacePresence::Present && registry == SurfacePresence::Present
            }
        };
        if !valid {
            break;
        }
        completed.push(*effect);
    }
    Ok(completed)
}
