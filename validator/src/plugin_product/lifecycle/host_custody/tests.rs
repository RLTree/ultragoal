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
    let custody = HostLifecycleCustody::take(plan.clone(), binding_for()).unwrap();
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

fn binding_for() -> HostLifecycleBinding {
    let package = PackageIdentity::new(
        SourceIdentity::new(
            digest('1'),
            digest('2'),
            "harness-ultragoal".to_owned(),
            "1.0.1".to_owned(),
            digest('3'),
            digest('4'),
        )
        .unwrap(),
        digest('5'),
        digest('6'),
    )
    .unwrap();
    let command_plan = HostCommandPlan::personal_install(&package, "local-marketplace").unwrap();
    HostLifecycleBinding::new(
        package,
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
