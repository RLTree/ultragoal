use super::HostEffectExecutionPolicy;
use super::lifecycle::{
    AcceptedHostState, AcceptedLifecycleOperation, AcceptedLifecyclePlan,
    AcceptedReconciliationPolicy, AcceptedRollbackPolicy, DescriptorExecutionAdapter,
    DescriptorExecutionCapability, DescriptorExecutionPlatform, DescriptorExecutionPrimitive,
    RootTrustedClock, SupportedHostLifecycleError, SupportedHostLifecycleErrorId,
    TrustedTimeSample, lifecycle_error,
};
use crate::distribution::PackageIdentity;
use crate::plugin_product::lifecycle::{HostLifecycleCustody, LifecycleIntent, LifecycleState};
use std::path::Path;
use std::time::SystemTime;

pub(crate) fn isolated_codex_policy(
    environment: &[(String, String)],
) -> Result<HostEffectExecutionPolicy, &'static str> {
    if environment.len() != 2
        || environment[0].0 != "CODEX_HOME"
        || environment[1].0 != "HOME"
        || environment[0].1 != environment[1].1
    {
        return Err("isolated Codex environment binding invalid");
    }
    let home = Path::new(&environment[0].1);
    let canonical = home
        .canonicalize()
        .map_err(|_| "isolated Codex home unavailable")?;
    if canonical.display().to_string() != environment[0].1 {
        return Err("isolated Codex home binding is not canonical");
    }
    HostEffectExecutionPolicy::strict_isolated_codex_home(30_000, home)
        .map_err(|_| "isolated Codex execution policy failed")
}

pub(crate) fn accepted_lifecycle(
    custody: &HostLifecycleCustody,
) -> Result<AcceptedLifecyclePlan, &'static str> {
    let state = |state: &LifecycleState| {
        match state.installed.as_ref() {
            None => AcceptedHostState::new(state.generation, None, state.recovery_required),
            Some(authority)
                if package_authority(custody.pre_effect_record().package())? == *authority =>
            {
                AcceptedHostState::new(
                    state.generation,
                    Some(custody.pre_effect_record().package().clone()),
                    state.recovery_required,
                )
            }
            Some(authority) => match custody.pre_effect_record().prior_authority() {
                Some(prior) if prior.authority() == authority => AcceptedHostState::observed_prior(
                    state.generation,
                    prior.clone(),
                    state.recovery_required,
                ),
                _ => return Err("accepted lifecycle package authority unavailable"),
            },
        }
        .map_err(|_| "accepted lifecycle state unavailable")
    };
    let before = state(custody.before())?;
    let after = state(custody.expected_after())?;
    let rollback = state(custody.before())?;
    let (operation, rollback_policy, reconciliation_policy) = policies(custody.intent());
    AcceptedLifecyclePlan::new(
        operation,
        before,
        after,
        rollback,
        rollback_policy,
        reconciliation_policy,
    )
    .map_err(|_| "accepted lifecycle plan unavailable")
}

fn package_authority(
    package: &PackageIdentity,
) -> Result<crate::plugin_product::lifecycle::PackageAuthority, &'static str> {
    let version = crate::plugin_product::lifecycle::Version::parse(package.source().version())
        .map_err(|_| "accepted lifecycle package version unavailable")?;
    Ok(crate::plugin_product::lifecycle::PackageAuthority {
        version,
        package_sha256: package.archive_sha256().to_owned(),
        inventory_sha256: package.source().accepted_inventory_sha256().to_owned(),
        candidate_id: package.source().candidate_id().to_owned(),
    })
}

pub(crate) fn accepted_operation(intent: LifecycleIntent) -> AcceptedLifecycleOperation {
    policies(intent).0
}

fn policies(
    intent: LifecycleIntent,
) -> (
    AcceptedLifecycleOperation,
    AcceptedRollbackPolicy,
    AcceptedReconciliationPolicy,
) {
    use AcceptedLifecycleOperation as O;
    use AcceptedReconciliationPolicy as R;
    use AcceptedRollbackPolicy as B;
    match intent {
        LifecycleIntent::FreshInstall => (
            O::FreshInstall,
            B::RemoveOnlyNewTarget,
            R::ExactPostStateAndSeparateHostLayers,
        ),
        LifecycleIntent::MonotonicUpdate => (
            O::MonotonicUpdate,
            B::RestoreExactPreState,
            R::ExactPostStateAndSeparateHostLayers,
        ),
        LifecycleIntent::FailedUpdateRecovery => (
            O::FailedUpdateRecovery,
            B::RestoreExactPreState,
            R::ExactPostStateAndSeparateHostLayers,
        ),
        LifecycleIntent::AuthorizedRollback => (
            O::AuthorizedRollback,
            B::RestoreExactPreState,
            R::ExactPostStateAndSeparateHostLayers,
        ),
        LifecycleIntent::IdempotentReinstall => (
            O::IdempotentReinstall,
            B::ManualReconciliationOnly,
            R::ExactPostStateAndSeparateHostLayers,
        ),
        LifecycleIntent::UninstallTeardown => (
            O::UninstallTeardown,
            B::RestoreExactPreState,
            R::ExactAbsenceAndSeparateHostLayers,
        ),
        LifecycleIntent::StaleCacheRecovery => (
            O::StaleCacheRecovery,
            B::RestoreExactPreState,
            R::ExactPostStateAndSeparateHostLayers,
        ),
        LifecycleIntent::RepeatUse => (
            O::RepeatUse,
            B::ManualReconciliationOnly,
            R::ExactPostStateAndSeparateHostLayers,
        ),
    }
}

pub(crate) struct CurrentDescriptorAdapter;

impl DescriptorExecutionAdapter for CurrentDescriptorAdapter {
    fn descriptor_capability(
        &mut self,
    ) -> Result<DescriptorExecutionCapability, SupportedHostLifecycleError> {
        current_capability()
    }
}

pub(crate) fn current_capability()
-> Result<DescriptorExecutionCapability, SupportedHostLifecycleError> {
    let (platform, primitive) = match DescriptorExecutionPlatform::current() {
        DescriptorExecutionPlatform::Darwin => (
            DescriptorExecutionPlatform::Darwin,
            DescriptorExecutionPrimitive::DarwinPosixSpawnSuspendedLoadedVnode,
        ),
        DescriptorExecutionPlatform::Linux => (
            DescriptorExecutionPlatform::Linux,
            DescriptorExecutionPrimitive::ExecveAtEmptyPath,
        ),
        DescriptorExecutionPlatform::FreeBsd => (
            DescriptorExecutionPlatform::FreeBsd,
            DescriptorExecutionPrimitive::Fexecve,
        ),
        DescriptorExecutionPlatform::Other => {
            return Err(lifecycle_error(
                SupportedHostLifecycleErrorId::DescriptorExecutionUnavailable,
            ));
        }
    };
    DescriptorExecutionCapability::new(
        platform,
        primitive,
        "harness-host-effect".into(),
        "v1".into(),
    )
}

#[derive(Default)]
pub(crate) struct SystemTrustedClock {
    sequence: u64,
}

impl RootTrustedClock for SystemTrustedClock {
    fn sample(&mut self) -> Result<TrustedTimeSample, SupportedHostLifecycleError> {
        self.sequence = self.sequence.saturating_add(1);
        let unix_ms = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| lifecycle_error(SupportedHostLifecycleErrorId::UntrustedTime))?
            .as_millis() as u64;
        TrustedTimeSample::new("system-clock".into(), 1, self.sequence, unix_ms)
    }
}

#[cfg(test)]
mod prior_authority_tests {
    use super::*;
    use crate::distribution::host_effect::lifecycle::{
        AcceptedHostState, AcceptedLifecycleOperation, AcceptedLifecyclePlan,
        AcceptedReconciliationPolicy, AcceptedRollbackPolicy,
    };
    use crate::distribution::{HostCommandPlan, SourceIdentity};
    use crate::plugin_product::lifecycle::{
        HostLifecycleBinding, HostLifecycleExpectedObservations, LifecycleAuthorization,
        LifecycleRequest, LifecycleState, PackageAuthority, Version, plan,
    };

    #[test]
    fn monotonic_update_preserves_distinct_prior_target_and_rollback_authority() {
        let prior = package("0.0.36", '6');
        let target = package("0.0.37", '7');
        let custody = custody(&prior, &target).unwrap();

        let accepted = accepted_lifecycle(&custody).unwrap();
        let prior_authority = observed_prior(&prior);
        let expected = AcceptedLifecyclePlan::new(
            AcceptedLifecycleOperation::MonotonicUpdate,
            AcceptedHostState::observed_prior(4, prior_authority.clone(), false).unwrap(),
            AcceptedHostState::new(5, Some(target.clone()), false).unwrap(),
            AcceptedHostState::observed_prior(4, prior_authority.clone(), false).unwrap(),
            AcceptedRollbackPolicy::RestoreExactPreState,
            AcceptedReconciliationPolicy::ExactPostStateAndSeparateHostLayers,
        )
        .unwrap();

        assert_eq!(accepted, expected);
        assert_eq!(
            custody.pre_effect_record().prior_authority(),
            Some(&prior_authority)
        );
    }

    #[test]
    fn monotonic_update_rejects_missing_substituted_and_target_as_prior_authority() {
        let prior = package("0.0.36", '6');
        let target = package("0.0.37", '7');
        assert!(custody_with_prior(&prior, &target, None).is_err());
        assert!(
            custody_with_prior(
                &prior,
                &target,
                Some(
                    crate::plugin_product::lifecycle::PriorInstalledAuthority::new(
                        PackageAuthority {
                            package_sha256: digest('5'),
                            ..authority(&prior)
                        },
                        digest('8'),
                    )
                    .unwrap()
                ),
            )
            .is_err()
        );
        assert!(custody_with_prior(&prior, &target, Some(observed_prior(&target))).is_err());
    }

    fn custody(
        prior: &PackageIdentity,
        target: &PackageIdentity,
    ) -> Result<HostLifecycleCustody, crate::plugin_product::lifecycle::LifecycleError> {
        custody_with_prior(prior, target, Some(observed_prior(prior)))
    }

    fn custody_with_prior(
        prior: &PackageIdentity,
        target: &PackageIdentity,
        prior_authority: Option<crate::plugin_product::lifecycle::PriorInstalledAuthority>,
    ) -> Result<HostLifecycleCustody, crate::plugin_product::lifecycle::LifecycleError> {
        let before = LifecycleState {
            installed: Some(authority(prior)),
            cache: Some(authority(prior)),
            generation: 4,
            recovery_required: false,
        };
        let lifecycle = plan(
            &before,
            &LifecycleRequest {
                intent: LifecycleIntent::MonotonicUpdate,
                target: Some(authority(target)),
                prior_authority: None,
                authorization: LifecycleAuthorization {
                    allow_host_write: true,
                    allow_downgrade: false,
                    expected_installed_sha256: Some(prior.archive_sha256().to_owned()),
                },
            },
        )?;
        HostLifecycleCustody::take(lifecycle, binding(target, prior_authority)?)
    }

    fn binding(
        target: &PackageIdentity,
        prior: Option<crate::plugin_product::lifecycle::PriorInstalledAuthority>,
    ) -> Result<HostLifecycleBinding, crate::plugin_product::lifecycle::LifecycleError> {
        let command_plan = HostCommandPlan::personal_install(target, "local-harness-plugins")
            .map_err(|_| crate::plugin_product::lifecycle::LifecycleError::InvalidTransition)?;
        HostLifecycleBinding::new(
            target.clone(),
            prior,
            command_plan,
            digest('a'),
            digest('b'),
            HostLifecycleExpectedObservations {
                installed_sha256: digest('c'),
                cache_sha256: digest('d'),
                registry_sha256: digest('e'),
                discovery_sha256: digest('f'),
                runtime_sha256: digest('1'),
                command_count: 1,
            },
        )
    }

    fn authority(package: &PackageIdentity) -> PackageAuthority {
        PackageAuthority {
            version: Version::parse(package.source().version()).unwrap(),
            package_sha256: package.archive_sha256().to_owned(),
            inventory_sha256: package.source().accepted_inventory_sha256().to_owned(),
            candidate_id: package.source().candidate_id().to_owned(),
        }
    }

    fn observed_prior(
        package: &PackageIdentity,
    ) -> crate::plugin_product::lifecycle::PriorInstalledAuthority {
        crate::plugin_product::lifecycle::PriorInstalledAuthority::new(
            authority(package),
            digest('8'),
        )
        .unwrap()
    }

    fn package(version: &str, archive_seed: char) -> PackageIdentity {
        PackageIdentity::new(
            SourceIdentity::new(
                digest('2'),
                digest(archive_seed),
                "harness-ultragoal".to_owned(),
                version.to_owned(),
                digest('3'),
                digest('4'),
            )
            .unwrap(),
            digest('5'),
            digest(archive_seed),
        )
        .unwrap()
    }

    fn digest(seed: char) -> String {
        format!("sha256:{}", seed.to_string().repeat(64))
    }
}
