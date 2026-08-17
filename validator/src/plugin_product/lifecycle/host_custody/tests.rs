use super::*;
use crate::distribution::{HostCommandPlan, PackageIdentity, SourceIdentity};
use crate::plugin_product::lifecycle::{
    LifecycleAuthorization, LifecycleEffect, LifecycleEffectAdapter, LifecycleIntent,
    LifecycleRequest, PackageAuthority, Version, apply, plan,
};

#[test]
fn transfer_replays_every_plan_clone_before_effect_execution() {
    let before = state('a');
    let plan = plan(
        &before,
        &LifecycleRequest {
            intent: LifecycleIntent::MonotonicUpdate,
            target: Some(authority('b', "1.0.1")),
            prior_authority: None,
            authorization: LifecycleAuthorization {
                allow_host_write: true,
                allow_downgrade: false,
                expected_installed_sha256: Some(digest('a')),
            },
        },
    )
    .unwrap();
    let replay = plan.clone();
    let custody = HostLifecycleCustody::take(plan.clone(), binding_for_update()).unwrap();
    let mut effects = NeverExecute;
    let record = custody.pre_effect_record();
    assert!(record.validate().is_ok());
    assert_eq!(record.plan_id, custody.plan.plan_id);
    assert_eq!(record.before, before);
    assert_eq!(record.effects, custody.effects());
    assert_eq!(
        apply(&before, &replay, &mut effects),
        Err(LifecycleError::ReplayedPlan)
    );
}

#[test]
fn custody_has_projection_only_before_single_command_consumption() {
    let custody = HostLifecycleCustody::take(
        plan(
            &LifecycleState::default(),
            &LifecycleRequest {
                intent: LifecycleIntent::FreshInstall,
                target: Some(authority('b', "1.0.1")),
                prior_authority: None,
                authorization: LifecycleAuthorization {
                    allow_host_write: true,
                    allow_downgrade: false,
                    expected_installed_sha256: None,
                },
            },
        )
        .unwrap(),
        binding_for_fresh(),
    )
    .unwrap();
    let source = include_str!("mod.rs");
    assert!(!source.contains("candidate_plan"));
    assert!(!source.contains("commit_release"));
    assert_eq!(custody.command_plan_projection().command_count(), 1);
}

#[test]
fn legacy_records_remain_readable_except_for_untyped_monotonic_updates() {
    let fresh = HostLifecycleCustody::take(
        plan(
            &LifecycleState::default(),
            &LifecycleRequest {
                intent: LifecycleIntent::FreshInstall,
                target: Some(authority('b', "1.0.1")),
                prior_authority: None,
                authorization: LifecycleAuthorization {
                    allow_host_write: true,
                    allow_downgrade: false,
                    expected_installed_sha256: None,
                },
            },
        )
        .unwrap(),
        binding_for_fresh(),
    )
    .unwrap();
    let mut legacy_fresh = fresh.pre_effect_record().clone();
    legacy_fresh.schema_version = "HarnessPluginHostLifecycleRecord-v2".to_owned();
    assert!(legacy_fresh.validate().is_ok());

    let before = state('a');
    let update = HostLifecycleCustody::take(
        plan(
            &before,
            &LifecycleRequest {
                intent: LifecycleIntent::MonotonicUpdate,
                target: Some(authority('b', "1.0.1")),
                prior_authority: None,
                authorization: LifecycleAuthorization {
                    allow_host_write: true,
                    allow_downgrade: false,
                    expected_installed_sha256: Some(digest('a')),
                },
            },
        )
        .unwrap(),
        binding_for_update(),
    )
    .unwrap();
    let mut legacy_update = update.pre_effect_record().clone();
    legacy_update.schema_version = "HarnessPluginHostLifecycleRecord-v2".to_owned();
    legacy_update.prior_authority = None;
    assert_eq!(
        legacy_update.validate(),
        Err(LifecycleError::MissingPriorAuthority)
    );
}

struct NeverExecute;

impl LifecycleEffectAdapter for NeverExecute {
    fn execute(&mut self, _: LifecycleEffect, _: &LifecycleState) -> Result<(), String> {
        panic!("replayed transfer reached effect execution")
    }

    fn restore(&mut self, _: &LifecycleState) -> Result<(), String> {
        panic!("replayed transfer reached recovery")
    }

    fn observe_state(&self) -> Result<LifecycleState, String> {
        panic!("replayed transfer reached observation")
    }
}

fn state(seed: char) -> LifecycleState {
    LifecycleState {
        installed: Some(authority(seed, "1.0.0")),
        cache: Some(authority(seed, "1.0.0")),
        generation: 1,
        recovery_required: false,
    }
}

fn authority(seed: char, version: &str) -> PackageAuthority {
    PackageAuthority {
        version: Version::parse(version).unwrap(),
        package_sha256: digest(seed),
        inventory_sha256: digest('c'),
        candidate_id: digest('d'),
    }
}

fn package_for(version: &str, archive_seed: char) -> PackageIdentity {
    PackageIdentity::new(
        SourceIdentity::new(
            digest('1'),
            digest('d'),
            "harness-ultragoal".to_owned(),
            version.to_owned(),
            digest('3'),
            digest('c'),
        )
        .unwrap(),
        digest('5'),
        digest(archive_seed),
    )
    .unwrap()
}

fn binding_for_fresh() -> HostLifecycleBinding {
    binding_for(package_for("1.0.1", 'b'), None)
}

fn binding_for_update() -> HostLifecycleBinding {
    binding_for(
        package_for("1.0.1", 'b'),
        Some(PriorInstalledAuthority::new(authority('a', "1.0.0"), digest('e')).unwrap()),
    )
}

fn binding_for(
    package: PackageIdentity,
    prior_authority: Option<PriorInstalledAuthority>,
) -> HostLifecycleBinding {
    let command_plan = HostCommandPlan::personal_install(&package, "local-marketplace").unwrap();
    HostLifecycleBinding::new(
        package,
        prior_authority,
        command_plan,
        digest('c'),
        digest('d'),
        HostLifecycleExpectedObservations {
            installed_sha256: digest('7'),
            cache_sha256: digest('8'),
            registry_sha256: digest('9'),
            discovery_sha256: digest('a'),
            runtime_sha256: digest('b'),
            command_count: 0,
        },
    )
    .unwrap()
}

fn digest(seed: char) -> String {
    format!("sha256:{}", seed.to_string().repeat(64))
}
