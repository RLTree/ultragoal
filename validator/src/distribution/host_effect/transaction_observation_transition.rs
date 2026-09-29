use super::transaction_observation::HostLifecycleObservationResult;
use crate::plugin_product::lifecycle::{LifecycleEffect, LifecyclePlan};

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

pub(crate) fn validate_read_only_transition(
    plan: &LifecyclePlan,
    result: &HostLifecycleObservationResult,
) -> Result<(), &'static str> {
    if plan.before.validate().is_err()
        || plan.expected_after.validate().is_err()
        || result.observed != plan.expected_after
        || result.completed_effects != plan.effects
    {
        return Err("read-only lifecycle transition validation failed");
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::distribution::host_effect::transaction_observation::HostLifecycleSurfaceDigests;
    use crate::plugin_product::lifecycle::{
        HostLifecycleObservedBundle, LifecycleAuthorization, LifecycleIntent, LifecycleRequest,
        LifecycleState, PackageAuthority, Version, plan,
    };

    fn digest(seed: char) -> String {
        format!("sha256:{}", seed.to_string().repeat(64))
    }

    fn read_only_plan() -> LifecyclePlan {
        let authority = PackageAuthority {
            version: Version::parse("1.0.0").unwrap(),
            package_sha256: digest('1'),
            inventory_sha256: digest('2'),
            candidate_id: digest('3'),
        };
        let before = LifecycleState {
            installed: Some(authority.clone()),
            cache: Some(authority),
            generation: 1,
            recovery_required: false,
        };
        plan(
            &before,
            &LifecycleRequest {
                intent: LifecycleIntent::RepeatUse,
                target: None,
                prior_authority: None,
                authorization: LifecycleAuthorization {
                    allow_host_write: false,
                    allow_downgrade: false,
                    expected_installed_sha256: None,
                },
            },
        )
        .unwrap()
    }

    fn complete_result(plan: &LifecyclePlan) -> HostLifecycleObservationResult {
        HostLifecycleObservationResult {
            observations: HostLifecycleObservedBundle::from_parts(
                digest('4'),
                digest('5'),
                digest('6'),
                digest('7'),
                digest('8'),
                Vec::new(),
            )
            .unwrap(),
            completed_effects: plan.effects.clone(),
            observed: plan.expected_after.clone(),
            effect_cursor: plan.effects.len(),
            surfaces: HostLifecycleSurfaceDigests {
                installed: digest('4'),
                cache: digest('5'),
                registry: digest('6'),
                runtime: digest('8'),
            },
        }
    }

    #[test]
    fn read_only_validation_is_repeatable_after_custody_admission() {
        let plan = read_only_plan();
        let result = complete_result(&plan);
        assert!(validate_read_only_transition(&plan, &result).is_ok());
        assert!(validate_read_only_transition(&plan, &result).is_ok());
    }

    #[test]
    fn read_only_validation_rejects_mismatched_post_state_without_consuming_input() {
        let plan = read_only_plan();
        let mut result = complete_result(&plan);
        result.completed_effects.pop();
        assert!(validate_read_only_transition(&plan, &result).is_err());
        result.completed_effects = plan.effects.clone();
        assert!(validate_read_only_transition(&plan, &result).is_ok());
    }
}
