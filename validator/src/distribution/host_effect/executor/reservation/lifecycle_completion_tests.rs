use super::*;
use crate::distribution::host_effect::{
    FileHostEffectLedger, HostEffectAuthority, HostEffectDecision, HostEffectPermitBinding,
};
use crate::plugin_product::lifecycle::{
    HostLifecycleCustody, LifecycleAuthorization, LifecycleIntent, LifecycleRequest,
    PackageAuthority, Version, plan,
};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

#[test]
fn reopen_reserve_in_flight_admits_only_the_signed_exact_record() {
    let mut custody = custody('a');
    let admission = admission(custody.pre_effect_record().clone()).unwrap();
    let binding = custody.begin_effects(admission).unwrap();
    let expected = custody.expected_after().clone();
    let effects = custody.effects().to_vec();
    assert!(
        custody
            .settle(HostEffectCompletion::settled(binding, expected, effects))
            .is_ok()
    );
}

#[test]
fn reopen_reserve_in_flight_rejects_record_substitution() {
    let owner = custody('a');
    let other = custody('b').pre_effect_record().clone();
    assert!(admission_for(owner.pre_effect_record().clone(), other).is_err());
}

#[test]
fn expired_permit_is_rejected_at_authority_current_time() {
    let record = custody('a').pre_effect_record().clone();
    let root = FixtureRoot::new();
    let path = root.path.join("ledger");
    let ledger = FileHostEffectLedger::create(&path, "fixture-ledger".to_owned()).unwrap();
    let now = unix_ms();
    let authority =
        HostEffectAuthority::generate("fixture-root".to_owned(), "fixture-ledger".to_owned())
            .unwrap();
    let (permit, _) = authority
        .issue(binding(record, &ledger.head().unwrap(), now - 2, now - 1))
        .unwrap();
    assert!(authority.verify_current(permit).is_err());
    drop(ledger);
    root.remove();
}

#[test]
fn ambiguous_terminal_observation_arms_recovery_after_durable_admission() {
    let mut custody = custody('a');
    let admission = admission(custody.pre_effect_record().clone()).unwrap();
    let binding = custody.begin_effects(admission).unwrap();
    let mut observed = custody.expected_after().clone();
    observed.cache = custody.before().cache.clone();
    observed.recovery_required = true;
    assert!(
        custody
            .settle(HostEffectCompletion::ambiguous(
                binding,
                observed,
                custody.effects()[..1].to_vec(),
            ))
            .is_ok()
    );
    assert!(custody.recovery_token().is_ok());
}

fn admission(
    record: HostLifecycleRecord,
) -> Result<DurableHostLifecycleAdmission, HostEffectLedgerError> {
    admission_for(record.clone(), record)
}

fn admission_for(
    bound: HostLifecycleRecord,
    admitted: HostLifecycleRecord,
) -> Result<DurableHostLifecycleAdmission, HostEffectLedgerError> {
    let root = FixtureRoot::new();
    let path = root.path.join("ledger");
    FileHostEffectLedger::create(&path, "fixture-ledger".to_owned()).unwrap();
    let ledger = FileHostEffectLedger::open(&path, "fixture-ledger".to_owned()).unwrap();
    let head = ledger.head().unwrap();
    let authority =
        HostEffectAuthority::generate("fixture-root".to_owned(), "fixture-ledger".to_owned())
            .unwrap();
    let now = unix_ms();
    let (permit, _) = authority
        .issue(binding(bound, &head, now, now + 1_000))
        .unwrap();
    let permit = authority.verify_current(permit).unwrap();
    let result = reserve_in_flight_lifecycle(&ledger, permit, admitted);
    drop(ledger);
    root.remove();
    result
}

fn binding(
    record: HostLifecycleRecord,
    head: &crate::distribution::host_effect::HostEffectLedgerHead,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
) -> HostEffectPermitBinding {
    let record_sha256 = record_digest(&record);
    let (plan_id, intent) = record.permit_join();
    HostEffectPermitBinding {
        context_id: digest('1'),
        candidate_id: digest('2'),
        package_identity_sha256: digest('3'),
        journey_binding_sha256: digest('4'),
        session_issuance_sha256: digest('5'),
        lifecycle_plan_sha256: plan_id.to_owned(),
        lifecycle_intent: intent_name(intent).to_owned(),
        expected_pre_state_sha256: digest('6'),
        expected_post_state_sha256: digest('7'),
        rollback_policy_sha256: digest('8'),
        reconciliation_policy_sha256: digest('9'),
        host_scope_sha256: digest('a'),
        host_capability_sha256: digest('b'),
        required_capabilities_sha256: digest('c'),
        external_request_sha256: digest('d'),
        command_plan_sha256: digest('e'),
        argv_sha256: digest('f'),
        executable_identity_sha256: digest('1'),
        target_identity_sha256: digest('2'),
        target_generation: 1,
        issued_at_unix_ms,
        expires_at_unix_ms,
        expected_head_sha256: head.head_sha256().to_owned(),
        lifecycle_record: Some(record),
        lifecycle_record_sha256: Some(record_sha256),
        decision: HostEffectDecision::Authorize,
    }
}

fn custody(seed: char) -> HostLifecycleCustody {
    let target_seed = if seed == 'a' { 'b' } else { 'e' };
    let before = crate::plugin_product::lifecycle::LifecycleState {
        installed: Some(authority(seed, "1.0.0")),
        cache: Some(authority(seed, "1.0.0")),
        generation: 1,
        recovery_required: false,
    };
    HostLifecycleCustody::take(
        plan(
            &before,
            &LifecycleRequest {
                intent: LifecycleIntent::MonotonicUpdate,
                target: Some(authority(target_seed, "1.0.1")),
                prior_authority: None,
                authorization: LifecycleAuthorization {
                    allow_host_write: true,
                    allow_downgrade: false,
                    expected_installed_sha256: Some(digest(seed)),
                },
            },
        )
        .unwrap(),
    )
    .unwrap()
}

fn authority(seed: char, version: &str) -> PackageAuthority {
    PackageAuthority {
        version: Version::parse(version).unwrap(),
        package_sha256: digest(seed),
        inventory_sha256: digest('c'),
        candidate_id: digest('d'),
    }
}

fn record_digest(record: &HostLifecycleRecord) -> String {
    format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(record).unwrap())
    )
}

fn intent_name(intent: LifecycleIntent) -> &'static str {
    match intent {
        LifecycleIntent::FreshInstall => "fresh_install",
        LifecycleIntent::MonotonicUpdate => "monotonic_update",
        LifecycleIntent::FailedUpdateRecovery => "failed_update_recovery",
        LifecycleIntent::AuthorizedRollback => "authorized_rollback",
        LifecycleIntent::IdempotentReinstall => "idempotent_reinstall",
        LifecycleIntent::UninstallTeardown => "uninstall_teardown",
        LifecycleIntent::StaleCacheRecovery => "stale_cache_recovery",
        LifecycleIntent::RepeatUse => "repeat_use",
    }
}

fn digest(seed: char) -> String {
    format!("sha256:{}", seed.to_string().repeat(64))
}

fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
        .try_into()
        .unwrap()
}

static FIXTURE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct FixtureRoot {
    path: PathBuf,
}

impl FixtureRoot {
    fn new() -> Self {
        let sequence = FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "ultragoal-lifecycle-completion-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        Self { path }
    }

    fn remove(self) {
        fs::remove_dir_all(self.path).unwrap();
    }
}
