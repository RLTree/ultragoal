use super::plugin_product::journey_matrix::{JOURNEYS, validate_journey_matrix};
use super::plugin_product::lifecycle::{
    ApplyDisposition, LifecycleAuthorization, LifecycleEffect, LifecycleEffectAdapter,
    LifecycleError, LifecycleIntent, LifecyclePlan, LifecycleRequest, LifecycleState,
    PackageAuthority, RecoveryToken, Version, apply, plan, recover, recovery_token, verify,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier};

const D0: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";
const D1: &str = "sha256:1111111111111111111111111111111111111111111111111111111111111111";
const D2: &str = "sha256:2222222222222222222222222222222222222222222222222222222222222222";
const D3: &str = "sha256:3333333333333333333333333333333333333333333333333333333333333333";

#[derive(Default)]
struct Adapter {
    effects: Vec<LifecycleEffect>,
    restored: Vec<LifecycleState>,
    fail_at: Option<LifecycleEffect>,
    fail_restore: bool,
    observe_calls: Cell<usize>,
}

impl LifecycleEffectAdapter for Adapter {
    fn execute(&mut self, effect: LifecycleEffect, _: &LifecycleState) -> Result<(), String> {
        self.effects.push(effect);
        if self.fail_at == Some(effect) {
            Err(format!("injected-{effect:?}"))
        } else {
            Ok(())
        }
    }

    fn restore(&mut self, prior: &LifecycleState) -> Result<(), String> {
        self.restored.push(prior.clone());
        if self.fail_restore {
            Err("injected-restore".to_owned())
        } else {
            Ok(())
        }
    }

    fn observe_state(&self) -> Result<LifecycleState, String> {
        self.observe_calls.set(self.observe_calls.get() + 1);
        Err("adapter-observation-not-configured".to_owned())
    }
}

struct BlockingAdapter {
    effects: Vec<LifecycleEffect>,
    entered: Arc<Barrier>,
    release: Arc<Barrier>,
}

impl LifecycleEffectAdapter for BlockingAdapter {
    fn execute(&mut self, effect: LifecycleEffect, _: &LifecycleState) -> Result<(), String> {
        self.effects.push(effect);
        if self.effects.len() == 1 {
            self.entered.wait();
            self.release.wait();
        }
        Ok(())
    }

    fn restore(&mut self, _: &LifecycleState) -> Result<(), String> {
        Err("blocking-adapter-restore-not-expected".to_owned())
    }

    fn observe_state(&self) -> Result<LifecycleState, String> {
        Err("blocking-adapter-observation-not-expected".to_owned())
    }
}

struct PartialFailureAdapter {
    attempted_effects: Vec<LifecycleEffect>,
    completed_effects: Vec<LifecycleEffect>,
    restored: Vec<LifecycleState>,
    current: LifecycleState,
    fail_at: LifecycleEffect,
    substitute_observed_candidate: bool,
}

impl PartialFailureAdapter {
    fn new(current: LifecycleState, fail_at: LifecycleEffect) -> Self {
        Self {
            attempted_effects: Vec::new(),
            completed_effects: Vec::new(),
            restored: Vec::new(),
            current,
            fail_at,
            substitute_observed_candidate: false,
        }
    }
}

impl LifecycleEffectAdapter for PartialFailureAdapter {
    fn execute(
        &mut self,
        effect: LifecycleEffect,
        expected_after: &LifecycleState,
    ) -> Result<(), String> {
        self.attempted_effects.push(effect);
        if effect == self.fail_at {
            self.current.recovery_required = true;
            return Err(format!("injected-{effect:?}"));
        }
        match effect {
            LifecycleEffect::InstallPackage => {
                self.current.installed = expected_after.installed.clone();
                self.current.generation = expected_after.generation;
            }
            LifecycleEffect::RefreshCache => {
                self.current.cache = expected_after.cache.clone();
                self.current.generation = expected_after.generation;
            }
            LifecycleEffect::RestorePriorAuthority => {
                self.current.installed = expected_after.installed.clone();
                self.current.cache = expected_after.cache.clone();
                self.current.generation = expected_after.generation;
            }
            LifecycleEffect::RemoveInstalledPackage => {
                self.current.installed = None;
                self.current.generation = expected_after.generation;
            }
            LifecycleEffect::RemoveCache => {
                self.current.cache = None;
                self.current.generation = expected_after.generation;
            }
            LifecycleEffect::VerifyInstalledBytes
            | LifecycleEffect::VerifyTeardown
            | LifecycleEffect::ProbeRuntime => {}
        }
        self.completed_effects.push(effect);
        Ok(())
    }

    fn restore(&mut self, prior: &LifecycleState) -> Result<(), String> {
        self.restored.push(prior.clone());
        Err("injected-restore".to_owned())
    }

    fn observe_state(&self) -> Result<LifecycleState, String> {
        let mut observed = self.current.clone();
        if self.substitute_observed_candidate {
            observed.installed.as_mut().unwrap().candidate_id = D2.to_owned();
        }
        Ok(observed)
    }
}

struct ZeroWriteRoot(PathBuf);

impl ZeroWriteRoot {
    fn new() -> Self {
        static NEXT_ROOT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "hul-plugin-read-failure-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(root.join("nested/empty")).unwrap();
        std::fs::write(root.join("nested/baseline.txt"), b"read-only baseline\n").unwrap();
        Self(root)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for ZeroWriteRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn recursive_snapshot(root: &Path) -> Vec<(PathBuf, &'static str, Vec<u8>)> {
    fn visit(root: &Path, path: &Path, rows: &mut Vec<(PathBuf, &'static str, Vec<u8>)>) {
        let metadata = std::fs::symlink_metadata(path).unwrap();
        let (kind, bytes) = if metadata.is_dir() {
            ("directory", Vec::new())
        } else if metadata.is_file() {
            ("file", std::fs::read(path).unwrap())
        } else {
            ("other", Vec::new())
        };
        rows.push((path.strip_prefix(root).unwrap().to_path_buf(), kind, bytes));
        if metadata.is_dir() {
            let mut entries = std::fs::read_dir(path)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .collect::<Vec<_>>();
            entries.sort();
            for entry in entries {
                visit(root, &entry, rows);
            }
        }
    }

    let mut rows = Vec::new();
    visit(root, root, &mut rows);
    rows
}

struct ZeroWriteTrapAdapter {
    root: PathBuf,
    fail_at: LifecycleEffect,
    effects: Vec<LifecycleEffect>,
    restore_calls: usize,
    observe_calls: Cell<usize>,
}

impl ZeroWriteTrapAdapter {
    fn new(root: &Path, fail_at: LifecycleEffect) -> Self {
        Self {
            root: root.to_path_buf(),
            fail_at,
            effects: Vec::new(),
            restore_calls: 0,
            observe_calls: Cell::new(0),
        }
    }
}

impl LifecycleEffectAdapter for ZeroWriteTrapAdapter {
    fn execute(&mut self, effect: LifecycleEffect, _: &LifecycleState) -> Result<(), String> {
        self.effects.push(effect);
        std::fs::read(self.root.join("nested/baseline.txt")).map_err(|error| error.to_string())?;
        if effect == self.fail_at {
            Err(format!("read-failed-{effect:?}"))
        } else {
            Ok(())
        }
    }

    fn restore(&mut self, _: &LifecycleState) -> Result<(), String> {
        self.restore_calls += 1;
        std::fs::write(self.root.join("restore-called"), b"unauthorized\n")
            .map_err(|error| error.to_string())
    }

    fn observe_state(&self) -> Result<LifecycleState, String> {
        self.observe_calls.set(self.observe_calls.get() + 1);
        std::fs::write(self.root.join("observe-called"), b"unauthorized\n")
            .map_err(|error| error.to_string())?;
        Err("unauthorized-observation".to_owned())
    }
}

fn authority(version: &str, digest: &str) -> PackageAuthority {
    PackageAuthority {
        version: Version::parse(version).unwrap(),
        package_sha256: digest.to_owned(),
        inventory_sha256: D0.to_owned(),
        candidate_id: D3.to_owned(),
    }
}

fn installed(item: PackageAuthority) -> LifecycleState {
    LifecycleState {
        installed: Some(item.clone()),
        cache: Some(item),
        generation: 7,
        recovery_required: false,
    }
}

fn auth(current: Option<&PackageAuthority>) -> LifecycleAuthorization {
    LifecycleAuthorization {
        allow_host_write: true,
        allow_downgrade: false,
        expected_installed_sha256: current.map(|item| item.package_sha256.clone()),
    }
}

fn request(
    intent: LifecycleIntent,
    target: Option<PackageAuthority>,
    authorization: LifecycleAuthorization,
) -> LifecycleRequest {
    LifecycleRequest {
        intent,
        target,
        prior_authority: None,
        authorization,
    }
}

fn recompute_public_plan_id(plan: &mut LifecyclePlan) {
    let bytes = serde_json::to_vec(&(
        plan.intent,
        &plan.before,
        &plan.expected_after,
        &plan.effects,
        plan.writes_host_state,
        &plan.authorization_sha256,
    ))
    .unwrap();
    plan.plan_id = format!("sha256:{:x}", Sha256::digest(bytes));
}

fn read_only_plan(intent: LifecycleIntent) -> (LifecycleState, LifecyclePlan) {
    let current = authority("0.0.12", D1);
    let state = installed(current.clone());
    let plan = plan(
        &state,
        &request(
            intent,
            Some(current),
            LifecycleAuthorization {
                allow_host_write: false,
                allow_downgrade: false,
                expected_installed_sha256: Some(D1.to_owned()),
            },
        ),
    )
    .unwrap();
    assert!(!plan.writes_host_state);
    (state, plan)
}

#[test]
fn journey_matrix_has_one_typed_entry_for_each_required_lifecycle_intent() {
    validate_journey_matrix().unwrap();
    assert_eq!(JOURNEYS.len(), 8);
    assert_eq!(
        JOURNEYS.iter().map(|item| item.intent).collect::<Vec<_>>(),
        vec![
            LifecycleIntent::FreshInstall,
            LifecycleIntent::MonotonicUpdate,
            LifecycleIntent::FailedUpdateRecovery,
            LifecycleIntent::AuthorizedRollback,
            LifecycleIntent::IdempotentReinstall,
            LifecycleIntent::UninstallTeardown,
            LifecycleIntent::StaleCacheRecovery,
            LifecycleIntent::RepeatUse,
        ]
    );
}

#[test]
fn fresh_install_plans_applies_and_verifies_exact_authority() {
    let before = LifecycleState::default();
    let target = authority("0.0.12", D1);
    let plan = plan(
        &before,
        &request(
            LifecycleIntent::FreshInstall,
            Some(target.clone()),
            auth(None),
        ),
    )
    .unwrap();
    assert!(plan.writes_host_state);
    assert_eq!(plan.expected_after.installed, Some(target.clone()));
    assert_eq!(plan.expected_after.cache, Some(target));
    let mut adapter = Adapter::default();
    let report = apply(&before, &plan, &mut adapter).unwrap();
    assert_eq!(report.disposition, ApplyDisposition::Applied);
    verify(&report.state, &plan).unwrap();
    assert_eq!(adapter.effects, plan.effects);
}

#[test]
fn monotonic_update_rejects_equal_or_older_and_recovers_prior_after_failure() {
    let current = authority("0.0.12", D1);
    let before = installed(current.clone());
    for target in [authority("0.0.12", D2), authority("0.0.11", D2)] {
        assert_eq!(
            plan(
                &before,
                &request(
                    LifecycleIntent::MonotonicUpdate,
                    Some(target),
                    auth(Some(&current))
                )
            ),
            Err(LifecycleError::InvalidTransition)
        );
    }
    let update = plan(
        &before,
        &request(
            LifecycleIntent::MonotonicUpdate,
            Some(authority("0.0.13", D2)),
            auth(Some(&current)),
        ),
    )
    .unwrap();
    let mut adapter = Adapter {
        fail_at: Some(LifecycleEffect::RefreshCache),
        ..Adapter::default()
    };
    let report = apply(&before, &update, &mut adapter).unwrap();
    assert_eq!(report.disposition, ApplyDisposition::RecoveredAfterFailure);
    assert_eq!(report.state, before);
    assert_eq!(adapter.restored, vec![before]);
    assert_eq!(report.failed_effect, Some(LifecycleEffect::RefreshCache));
}

#[test]
fn explicit_failed_update_recovery_is_prior_bound() {
    let prior = installed(authority("0.0.12", D1));
    let interrupted = LifecycleState {
        installed: Some(authority("0.0.13", D2)),
        cache: prior.cache.clone(),
        generation: 8,
        recovery_required: true,
    };
    let recovery = LifecycleRequest {
        intent: LifecycleIntent::FailedUpdateRecovery,
        target: None,
        prior_authority: Some(prior.clone()),
        authorization: LifecycleAuthorization {
            allow_host_write: true,
            allow_downgrade: true,
            expected_installed_sha256: Some(D2.to_owned()),
        },
    };
    let plan = plan(&interrupted, &recovery).unwrap();
    assert_eq!(plan.expected_after.installed, prior.installed);
    assert!(!plan.expected_after.recovery_required);
    let mut adapter = Adapter::default();
    let report = apply(&interrupted, &plan, &mut adapter).unwrap();
    verify(&report.state, &plan).unwrap();
}

#[test]
fn downgrade_requires_specific_authority_and_expected_prior() {
    let current = authority("0.0.13", D2);
    let before = installed(current.clone());
    let mut authorization = auth(Some(&current));
    let rollback_request = request(
        LifecycleIntent::AuthorizedRollback,
        Some(authority("0.0.12", D1)),
        authorization.clone(),
    );
    assert_eq!(
        plan(&before, &rollback_request),
        Err(LifecycleError::DowngradeAuthorizationRequired)
    );
    authorization.allow_downgrade = true;
    let rollback = plan(
        &before,
        &request(
            LifecycleIntent::AuthorizedRollback,
            Some(authority("0.0.12", D1)),
            authorization,
        ),
    )
    .unwrap();
    assert_eq!(
        rollback.expected_after.installed.unwrap().package_sha256,
        D1
    );
}

#[test]
fn reinstall_is_idempotent_and_read_only_but_rejects_mismatch() {
    let current = authority("0.0.12", D1);
    let before = installed(current.clone());
    let reinstall = plan(
        &before,
        &request(
            LifecycleIntent::IdempotentReinstall,
            Some(current.clone()),
            LifecycleAuthorization {
                allow_host_write: false,
                allow_downgrade: false,
                expected_installed_sha256: Some(D1.to_owned()),
            },
        ),
    )
    .unwrap();
    assert!(!reinstall.writes_host_state);
    assert_eq!(reinstall.expected_after, before);
    assert_eq!(
        plan(
            &before,
            &request(
                LifecycleIntent::IdempotentReinstall,
                Some(authority("0.0.12", D2)),
                auth(Some(&current)),
            )
        ),
        Err(LifecycleError::InvalidTransition)
    );
}

#[test]
fn uninstall_teardown_removes_installed_and_cache_authority() {
    let current = authority("0.0.12", D1);
    let before = installed(current.clone());
    let teardown = plan(
        &before,
        &request(
            LifecycleIntent::UninstallTeardown,
            None,
            auth(Some(&current)),
        ),
    )
    .unwrap();
    assert_eq!(teardown.expected_after.installed, None);
    assert_eq!(teardown.expected_after.cache, None);
    let mut adapter = Adapter::default();
    let report = apply(&before, &teardown, &mut adapter).unwrap();
    verify(&report.state, &teardown).unwrap();
    assert_eq!(
        adapter.effects.last(),
        Some(&LifecycleEffect::VerifyTeardown)
    );
}

#[test]
fn stale_cache_recovery_preserves_installed_authority() {
    let current = authority("0.0.13", D2);
    let before = LifecycleState {
        installed: Some(current.clone()),
        cache: Some(authority("0.0.12", D1)),
        generation: 9,
        recovery_required: false,
    };
    let recovery = plan(
        &before,
        &request(
            LifecycleIntent::StaleCacheRecovery,
            None,
            auth(Some(&current)),
        ),
    )
    .unwrap();
    assert_eq!(recovery.expected_after.installed, Some(current.clone()));
    assert_eq!(recovery.expected_after.cache, Some(current));
}

#[test]
fn repeat_use_is_zero_write_and_rejects_candidate_substitution() {
    let current = authority("0.0.12", D1);
    let before = installed(current.clone());
    let reuse = plan(
        &before,
        &request(
            LifecycleIntent::RepeatUse,
            Some(current.clone()),
            LifecycleAuthorization {
                allow_host_write: false,
                allow_downgrade: false,
                expected_installed_sha256: Some(D1.to_owned()),
            },
        ),
    )
    .unwrap();
    assert!(!reuse.writes_host_state);
    assert_eq!(reuse.expected_after, before);
    assert_eq!(
        plan(
            &before,
            &request(
                LifecycleIntent::RepeatUse,
                Some(authority("0.0.12", D2)),
                auth(Some(&current)),
            )
        ),
        Err(LifecycleError::ExpectedPriorMismatch)
    );
}

#[test]
fn every_read_only_effect_failure_is_causal_and_closes_without_recovery_calls() {
    for intent in [
        LifecycleIntent::RepeatUse,
        LifecycleIntent::IdempotentReinstall,
    ] {
        for fail_at in [
            LifecycleEffect::VerifyInstalledBytes,
            LifecycleEffect::ProbeRuntime,
        ] {
            let (state, read_only) = read_only_plan(intent);
            let replay = read_only.clone();
            let mut adapter = Adapter {
                fail_at: Some(fail_at),
                ..Adapter::default()
            };
            assert_eq!(
                apply(&state, &read_only, &mut adapter),
                Err(LifecycleError::ReadEffectFailed {
                    effect: fail_at,
                    causal_error: format!("injected-{fail_at:?}"),
                })
            );
            let failed_index = read_only
                .effects
                .iter()
                .position(|effect| *effect == fail_at)
                .unwrap();
            assert_eq!(adapter.effects, read_only.effects[..=failed_index]);
            assert!(adapter.restored.is_empty());
            assert_eq!(adapter.observe_calls.get(), 0);
            assert_eq!(
                recovery_token(&read_only),
                Err(LifecycleError::InvalidTransition)
            );

            let mut replay_adapter = Adapter::default();
            assert_eq!(
                apply(&state, &replay, &mut replay_adapter),
                Err(LifecycleError::ReplayedPlan)
            );
            assert!(replay_adapter.effects.is_empty());
            assert!(replay_adapter.restored.is_empty());
            assert_eq!(replay_adapter.observe_calls.get(), 0);
        }
    }
}

#[test]
fn concurrent_read_only_failure_has_one_causal_path_and_no_restore_path() {
    let (state, read_only) = read_only_plan(LifecycleIntent::RepeatUse);
    let barrier = Arc::new(Barrier::new(2));
    let handles = (0..2)
        .map(|_| {
            let state = state.clone();
            let read_only = read_only.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                let mut adapter = Adapter {
                    fail_at: Some(LifecycleEffect::VerifyInstalledBytes),
                    ..Adapter::default()
                };
                barrier.wait();
                let result = apply(&state, &read_only, &mut adapter);
                (
                    result,
                    adapter.effects.len(),
                    adapter.restored.len(),
                    adapter.observe_calls.get(),
                )
            })
        })
        .collect::<Vec<_>>();
    let results = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        results
            .iter()
            .filter(|(result, _, _, _)| {
                *result
                    == Err(LifecycleError::ReadEffectFailed {
                        effect: LifecycleEffect::VerifyInstalledBytes,
                        causal_error: "injected-VerifyInstalledBytes".to_owned(),
                    })
            })
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|(result, _, _, _)| *result == Err(LifecycleError::ReplayedPlan))
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .map(|(_, effect_count, _, _)| effect_count)
            .sum::<usize>(),
        1
    );
    assert!(results.iter().all(|(_, _, restore_count, observe_count)| {
        *restore_count == 0 && *observe_count == 0
    }));
}

#[test]
fn serialized_mutated_read_only_plan_refuses_before_every_adapter_call() {
    let (state, read_only) = read_only_plan(LifecycleIntent::IdempotentReinstall);
    let mut transported: LifecyclePlan =
        serde_json::from_slice(&serde_json::to_vec(&read_only).unwrap()).unwrap();
    transported.effects.insert(0, LifecycleEffect::RemoveCache);
    transported.writes_host_state = true;
    recompute_public_plan_id(&mut transported);
    let mut adapter = Adapter {
        fail_at: Some(LifecycleEffect::VerifyInstalledBytes),
        ..Adapter::default()
    };
    assert_eq!(
        apply(&state, &transported, &mut adapter),
        Err(LifecycleError::UnsealedPlan)
    );
    assert!(adapter.effects.is_empty());
    assert!(adapter.restored.is_empty());
    assert_eq!(adapter.observe_calls.get(), 0);
}

#[test]
fn repeat_use_and_reinstall_failures_preserve_the_recursive_tree_exactly() {
    let root = ZeroWriteRoot::new();
    let baseline = recursive_snapshot(root.path());
    for intent in [
        LifecycleIntent::RepeatUse,
        LifecycleIntent::IdempotentReinstall,
    ] {
        for fail_at in [
            LifecycleEffect::VerifyInstalledBytes,
            LifecycleEffect::ProbeRuntime,
        ] {
            let (state, read_only) = read_only_plan(intent);
            let before = recursive_snapshot(root.path());
            let mut adapter = ZeroWriteTrapAdapter::new(root.path(), fail_at);
            assert_eq!(
                apply(&state, &read_only, &mut adapter),
                Err(LifecycleError::ReadEffectFailed {
                    effect: fail_at,
                    causal_error: format!("read-failed-{fail_at:?}"),
                })
            );
            assert_eq!(adapter.restore_calls, 0);
            assert_eq!(adapter.observe_calls.get(), 0);
            assert_eq!(recursive_snapshot(root.path()), before);
        }
    }
    assert_eq!(recursive_snapshot(root.path()), baseline);
}

#[test]
fn stale_plan_race_is_rejected_before_any_effect() {
    let current = authority("0.0.12", D1);
    let before = installed(current.clone());
    let update = plan(
        &before,
        &request(
            LifecycleIntent::MonotonicUpdate,
            Some(authority("0.0.13", D2)),
            auth(Some(&current)),
        ),
    )
    .unwrap();
    let mut drifted = before;
    drifted.generation += 1;
    let mut adapter = Adapter::default();
    assert_eq!(
        apply(&drifted, &update, &mut adapter),
        Err(LifecycleError::StalePlan)
    );
    assert!(adapter.effects.is_empty());
}

#[test]
fn plan_identity_binds_authorization_and_rejects_post_plan_mutation() {
    let current = authority("0.0.12", D1);
    let before = installed(current.clone());
    let mut update = plan(
        &before,
        &request(
            LifecycleIntent::MonotonicUpdate,
            Some(authority("0.0.13", D2)),
            auth(Some(&current)),
        ),
    )
    .unwrap();
    let original_id = update.plan_id.clone();
    update.expected_after.generation += 1;
    let mut adapter = Adapter::default();
    assert_eq!(
        apply(&before, &update, &mut adapter),
        Err(LifecycleError::InvalidTransition)
    );
    assert!(adapter.effects.is_empty());
    assert_eq!(original_id.len(), "sha256:".len() + 64);
}

#[test]
fn successful_apply_arms_exactly_one_explicit_recovery() {
    let current = authority("0.0.12", D1);
    let before = installed(current.clone());
    let update = plan(
        &before,
        &request(
            LifecycleIntent::MonotonicUpdate,
            Some(authority("0.0.13", D2)),
            auth(Some(&current)),
        ),
    )
    .unwrap();
    let token = recovery_token(&update).unwrap();
    let mut apply_adapter = Adapter::default();
    let report = apply(&before, &update, &mut apply_adapter).unwrap();
    assert_eq!(report.state, update.expected_after);
    let mut adapter = Adapter::default();
    assert_eq!(
        recover(&report.state, &token, &mut adapter).unwrap(),
        before
    );
    assert_eq!(adapter.restored, vec![before]);
}

#[test]
fn failed_effect_and_failed_automatic_restore_preserve_one_recovery_action() {
    let current = authority("0.0.12", D1);
    let before = installed(current.clone());
    let update = plan(
        &before,
        &request(
            LifecycleIntent::MonotonicUpdate,
            Some(authority("0.0.13", D2)),
            auth(Some(&current)),
        ),
    )
    .unwrap();
    let token = recovery_token(&update).unwrap();
    let mut failing = PartialFailureAdapter::new(before.clone(), LifecycleEffect::RefreshCache);
    assert_eq!(
        apply(&before, &update, &mut failing),
        Err(LifecycleError::RecoveryFailed(
            "injected-restore".to_owned()
        ))
    );
    assert_eq!(failing.restored, vec![before.clone()]);
    assert_eq!(
        failing.completed_effects,
        vec![LifecycleEffect::InstallPackage]
    );
    assert_eq!(
        failing.attempted_effects,
        vec![
            LifecycleEffect::InstallPackage,
            LifecycleEffect::RefreshCache,
        ]
    );
    let partial = failing.observe_state().unwrap();
    assert_ne!(partial, update.expected_after);
    assert_eq!(partial.installed, update.expected_after.installed);
    assert_eq!(partial.cache, before.cache);
    assert_eq!(partial.generation, update.expected_after.generation);
    assert!(partial.recovery_required);

    let mut planned_final = Adapter::default();
    assert_eq!(
        recover(&update.expected_after, &token, &mut planned_final),
        Err(LifecycleError::StaleRecoveryToken)
    );
    assert!(planned_final.restored.is_empty());

    let mut newer_generation = partial.clone();
    newer_generation.generation += 1;
    let mut newer_adapter = Adapter::default();
    assert_eq!(
        recover(&newer_generation, &token, &mut newer_adapter),
        Err(LifecycleError::StaleRecoveryToken)
    );
    assert!(newer_adapter.restored.is_empty());

    let mut substituted_candidate = partial.clone();
    substituted_candidate
        .installed
        .as_mut()
        .unwrap()
        .candidate_id = D2.to_owned();
    let mut substituted_adapter = Adapter::default();
    assert_eq!(
        recover(&substituted_candidate, &token, &mut substituted_adapter),
        Err(LifecycleError::StaleRecoveryToken)
    );
    assert!(substituted_adapter.restored.is_empty());

    let mut recovery = Adapter::default();
    assert_eq!(recover(&partial, &token, &mut recovery).unwrap(), before);
    assert_eq!(recovery.restored, vec![before]);
}

#[test]
fn unmodeled_failed_restore_observation_closes_recovery_authority() {
    let current = authority("0.0.12", D1);
    let before = installed(current.clone());
    let update = plan(
        &before,
        &request(
            LifecycleIntent::MonotonicUpdate,
            Some(authority("0.0.13", D2)),
            auth(Some(&current)),
        ),
    )
    .unwrap();
    let token = recovery_token(&update).unwrap();
    let mut failing = PartialFailureAdapter::new(before.clone(), LifecycleEffect::RefreshCache);
    failing.substitute_observed_candidate = true;
    assert_eq!(
        apply(&before, &update, &mut failing),
        Err(LifecycleError::RecoveryStateMismatch)
    );
    let legitimate_partial = failing.current.clone();
    let mut refused = Adapter::default();
    assert_eq!(
        recover(&legitimate_partial, &token, &mut refused),
        Err(LifecycleError::ReplayedRecoveryToken)
    );
    assert!(refused.restored.is_empty());
}

#[test]
fn host_mutations_fail_closed_without_authorization() {
    let target = authority("0.0.12", D1);
    assert_eq!(
        plan(
            &LifecycleState::default(),
            &request(
                LifecycleIntent::FreshInstall,
                Some(target),
                LifecycleAuthorization {
                    allow_host_write: false,
                    allow_downgrade: false,
                    expected_installed_sha256: None,
                },
            )
        ),
        Err(LifecycleError::AuthorizationRequired)
    );
}

#[test]
fn serialized_plan_is_transport_only_and_cannot_recover_apply_authority() {
    let current = authority("0.0.12", D1);
    let before = installed(current.clone());
    let authorized = plan(
        &before,
        &request(
            LifecycleIntent::MonotonicUpdate,
            Some(authority("0.0.13", D2)),
            auth(Some(&current)),
        ),
    )
    .unwrap();
    let transported: LifecyclePlan =
        serde_json::from_slice(&serde_json::to_vec(&authorized).unwrap()).unwrap();
    let mut adapter = Adapter::default();
    assert_eq!(
        apply(&before, &transported, &mut adapter),
        Err(LifecycleError::UnsealedPlan)
    );
    assert!(adapter.effects.is_empty());
    assert!(adapter.restored.is_empty());
}

#[test]
fn effect_expansion_cannot_turn_a_read_only_plan_into_write_authority() {
    let current = authority("0.0.12", D1);
    let before = installed(current.clone());
    let mut forged = plan(
        &before,
        &request(
            LifecycleIntent::RepeatUse,
            Some(current),
            LifecycleAuthorization {
                allow_host_write: false,
                allow_downgrade: false,
                expected_installed_sha256: Some(D1.to_owned()),
            },
        ),
    )
    .unwrap();
    forged.effects.insert(0, LifecycleEffect::RemoveCache);
    forged.writes_host_state = true;
    recompute_public_plan_id(&mut forged);
    let mut adapter = Adapter::default();
    assert_eq!(
        apply(&before, &forged, &mut adapter),
        Err(LifecycleError::InvalidTransition)
    );
    assert!(adapter.effects.is_empty());
    assert!(adapter.restored.is_empty());
}

#[test]
fn caller_write_flag_cannot_override_effect_derived_classification() {
    let current = authority("0.0.12", D1);
    let before = installed(current.clone());
    let mut forged = plan(
        &before,
        &request(
            LifecycleIntent::MonotonicUpdate,
            Some(authority("0.0.13", D2)),
            auth(Some(&current)),
        ),
    )
    .unwrap();
    forged.writes_host_state = false;
    recompute_public_plan_id(&mut forged);
    let mut adapter = Adapter::default();
    assert_eq!(
        apply(&before, &forged, &mut adapter),
        Err(LifecycleError::InvalidTransition)
    );
    assert!(adapter.effects.is_empty());
    assert!(adapter.restored.is_empty());
}

#[test]
fn recomputed_digest_cannot_substitute_the_authorized_target() {
    let current = authority("0.0.12", D1);
    let before = installed(current.clone());
    let mut substituted = plan(
        &before,
        &request(
            LifecycleIntent::MonotonicUpdate,
            Some(authority("0.0.13", D2)),
            auth(Some(&current)),
        ),
    )
    .unwrap();
    let replacement = authority("0.0.13", D3);
    substituted.expected_after.installed = Some(replacement.clone());
    substituted.expected_after.cache = Some(replacement);
    recompute_public_plan_id(&mut substituted);
    let mut adapter = Adapter::default();
    assert_eq!(
        apply(&before, &substituted, &mut adapter),
        Err(LifecycleError::InvalidTransition)
    );
    assert!(adapter.effects.is_empty());
    assert!(adapter.restored.is_empty());
}

#[test]
fn applied_plan_and_every_clone_are_replay_protected_before_effects() {
    let current = authority("0.0.12", D1);
    let before = installed(current.clone());
    let update = plan(
        &before,
        &request(
            LifecycleIntent::MonotonicUpdate,
            Some(authority("0.0.13", D2)),
            auth(Some(&current)),
        ),
    )
    .unwrap();
    let replay = update.clone();
    let mut first = Adapter::default();
    apply(&before, &update, &mut first).unwrap();
    let mut second = Adapter::default();
    assert_eq!(
        apply(&before, &replay, &mut second),
        Err(LifecycleError::ReplayedPlan)
    );
    assert!(second.effects.is_empty());
    assert!(second.restored.is_empty());
}

#[test]
fn concurrent_plan_replay_race_executes_exactly_one_authorized_transition() {
    let current = authority("0.0.12", D1);
    let before = installed(current.clone());
    let update = plan(
        &before,
        &request(
            LifecycleIntent::MonotonicUpdate,
            Some(authority("0.0.13", D2)),
            auth(Some(&current)),
        ),
    )
    .unwrap();
    let expected_effect_count = update.effects.len();
    let barrier = Arc::new(Barrier::new(2));
    let handles = (0..2)
        .map(|_| {
            let barrier = Arc::clone(&barrier);
            let before = before.clone();
            let update = update.clone();
            std::thread::spawn(move || {
                let mut adapter = Adapter::default();
                barrier.wait();
                let result = apply(&before, &update, &mut adapter);
                (result, adapter.effects.len(), adapter.restored.len())
            })
        })
        .collect::<Vec<_>>();
    let results = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        results
            .iter()
            .filter(|(result, _, _)| result.is_ok())
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|(result, _, _)| *result == Err(LifecycleError::ReplayedPlan))
            .count(),
        1
    );
    assert_eq!(
        results.iter().map(|(_, effects, _)| effects).sum::<usize>(),
        expected_effect_count
    );
    assert!(results.iter().all(|(_, _, restored)| *restored == 0));
}

#[test]
fn serialized_recovery_token_cannot_recover_restore_authority() {
    let current = authority("0.0.12", D1);
    let before = installed(current.clone());
    let update = plan(
        &before,
        &request(
            LifecycleIntent::MonotonicUpdate,
            Some(authority("0.0.13", D2)),
            auth(Some(&current)),
        ),
    )
    .unwrap();
    let authorized = recovery_token(&update).unwrap();
    let mut apply_adapter = Adapter::default();
    apply(&before, &update, &mut apply_adapter).unwrap();
    let transported: RecoveryToken =
        serde_json::from_slice(&serde_json::to_vec(&authorized).unwrap()).unwrap();
    let mut adapter = Adapter::default();
    assert_eq!(
        recover(&update.expected_after, &transported, &mut adapter),
        Err(LifecycleError::UnsealedRecoveryToken)
    );
    assert!(adapter.effects.is_empty());
    assert!(adapter.restored.is_empty());
}

#[test]
fn recovery_token_rejects_restore_substitution_before_adapter_effects() {
    let current = authority("0.0.12", D1);
    let before = installed(current.clone());
    let update = plan(
        &before,
        &request(
            LifecycleIntent::MonotonicUpdate,
            Some(authority("0.0.13", D2)),
            auth(Some(&current)),
        ),
    )
    .unwrap();
    let mut substituted = recovery_token(&update).unwrap();
    substituted.prior = installed(authority("0.0.11", D3));
    let mut adapter = Adapter::default();
    assert_eq!(
        recover(&update.expected_after, &substituted, &mut adapter),
        Err(LifecycleError::InvalidTransition)
    );
    assert!(adapter.effects.is_empty());
    assert!(adapter.restored.is_empty());
    let mut expected_current_substitution = recovery_token(&update).unwrap();
    expected_current_substitution.expected_current.generation += 1;
    let mut second_adapter = Adapter::default();
    assert_eq!(
        recover(
            &update.expected_after,
            &expected_current_substitution,
            &mut second_adapter
        ),
        Err(LifecycleError::InvalidTransition)
    );
    assert!(second_adapter.effects.is_empty());
    assert!(second_adapter.restored.is_empty());
}

#[test]
fn stale_recovery_authority_refuses_without_consuming_the_valid_restore() {
    let current = authority("0.0.12", D1);
    let before = installed(current.clone());
    let update = plan(
        &before,
        &request(
            LifecycleIntent::MonotonicUpdate,
            Some(authority("0.0.13", D2)),
            auth(Some(&current)),
        ),
    )
    .unwrap();
    let token = recovery_token(&update).unwrap();
    let mut apply_adapter = Adapter::default();
    apply(&before, &update, &mut apply_adapter).unwrap();
    let mut stale = update.expected_after.clone();
    stale.generation += 1;
    let mut stale_adapter = Adapter::default();
    assert_eq!(
        recover(&stale, &token, &mut stale_adapter),
        Err(LifecycleError::StaleRecoveryToken)
    );
    assert!(stale_adapter.restored.is_empty());
    let mut authorized_adapter = Adapter::default();
    assert_eq!(
        recover(&update.expected_after, &token, &mut authorized_adapter).unwrap(),
        before
    );
    assert_eq!(authorized_adapter.restored, vec![before]);
}

#[test]
fn recovery_before_apply_refuses_without_consuming_later_recovery() {
    let current = authority("0.0.12", D1);
    let before = installed(current.clone());
    let update = plan(
        &before,
        &request(
            LifecycleIntent::MonotonicUpdate,
            Some(authority("0.0.13", D2)),
            auth(Some(&current)),
        ),
    )
    .unwrap();
    let token = recovery_token(&update).unwrap();
    let mut premature = Adapter::default();
    assert_eq!(
        recover(&update.expected_after, &token, &mut premature),
        Err(LifecycleError::RecoveryUnavailable)
    );
    assert!(premature.restored.is_empty());
    let mut apply_adapter = Adapter::default();
    let report = apply(&before, &update, &mut apply_adapter).unwrap();
    let mut recovery = Adapter::default();
    assert_eq!(
        recover(&report.state, &token, &mut recovery).unwrap(),
        before
    );
    assert_eq!(recovery.restored, vec![before]);
}

#[test]
fn duplicate_tokens_and_clones_share_exactly_one_post_apply_recovery() {
    let current = authority("0.0.12", D1);
    let before = installed(current.clone());
    let update = plan(
        &before,
        &request(
            LifecycleIntent::MonotonicUpdate,
            Some(authority("0.0.13", D2)),
            auth(Some(&current)),
        ),
    )
    .unwrap();
    let token = recovery_token(&update).unwrap();
    let duplicate = recovery_token(&update).unwrap();
    let cloned = token.clone();
    let mut apply_adapter = Adapter::default();
    let report = apply(&before, &update, &mut apply_adapter).unwrap();
    let mut first = Adapter::default();
    recover(&report.state, &token, &mut first).unwrap();
    for replay in [&duplicate, &cloned] {
        let mut adapter = Adapter::default();
        assert_eq!(
            recover(&report.state, replay, &mut adapter),
            Err(LifecycleError::ReplayedRecoveryToken)
        );
        assert!(adapter.restored.is_empty());
    }
    let mut replayed_apply = Adapter::default();
    assert_eq!(
        apply(&before, &update, &mut replayed_apply),
        Err(LifecycleError::ReplayedPlan)
    );
    assert!(replayed_apply.effects.is_empty());
}

#[test]
fn successful_automatic_restore_closes_redundant_recovery() {
    let current = authority("0.0.12", D1);
    let before = installed(current.clone());
    let update = plan(
        &before,
        &request(
            LifecycleIntent::MonotonicUpdate,
            Some(authority("0.0.13", D2)),
            auth(Some(&current)),
        ),
    )
    .unwrap();
    let token = recovery_token(&update).unwrap();
    let mut failing = Adapter {
        fail_at: Some(LifecycleEffect::RefreshCache),
        ..Adapter::default()
    };
    let report = apply(&before, &update, &mut failing).unwrap();
    assert_eq!(report.disposition, ApplyDisposition::RecoveredAfterFailure);
    assert_eq!(report.state, before);
    let mut redundant = Adapter::default();
    assert_eq!(
        recover(&update.expected_after, &token, &mut redundant),
        Err(LifecycleError::ReplayedRecoveryToken)
    );
    assert!(redundant.restored.is_empty());
}

#[test]
fn recovery_during_apply_refuses_until_effects_finish_and_then_arms() {
    let current = authority("0.0.12", D1);
    let before = installed(current.clone());
    let update = plan(
        &before,
        &request(
            LifecycleIntent::MonotonicUpdate,
            Some(authority("0.0.13", D2)),
            auth(Some(&current)),
        ),
    )
    .unwrap();
    let token = recovery_token(&update).unwrap();
    let entered = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let apply_thread = {
        let before = before.clone();
        let update = update.clone();
        let entered = Arc::clone(&entered);
        let release = Arc::clone(&release);
        std::thread::spawn(move || {
            let mut adapter = BlockingAdapter {
                effects: Vec::new(),
                entered,
                release,
            };
            let report = apply(&before, &update, &mut adapter);
            (report, adapter.effects)
        })
    };
    entered.wait();
    let mut concurrent = Adapter::default();
    assert_eq!(
        recover(&update.expected_after, &token, &mut concurrent),
        Err(LifecycleError::RecoveryUnavailable)
    );
    assert!(concurrent.restored.is_empty());
    release.wait();
    let (report, effects) = apply_thread.join().unwrap();
    let report = report.unwrap();
    assert_eq!(effects, update.effects);
    let mut recovery = Adapter::default();
    assert_eq!(
        recover(&report.state, &token, &mut recovery).unwrap(),
        before
    );
    assert_eq!(recovery.restored, vec![before]);
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LifecycleCases {
    schema_version: String,
    cases: Vec<LifecycleCase>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LifecycleCase {
    id: String,
    intent: LifecycleIntent,
    expected_effects: Vec<LifecycleEffect>,
    prior_authority_preserved_on_failure: bool,
}

#[test]
fn lifecycle_fixture_is_exact_and_covers_all_eight_intents() {
    let fixture: LifecycleCases =
        serde_json::from_str(&super::read("fixtures/plugin-product/lifecycle-cases.json")).unwrap();
    assert_eq!(fixture.schema_version, "HarnessPluginLifecycleCases-v1");
    assert_eq!(fixture.cases.len(), 8);
    for (case, journey) in fixture.cases.iter().zip(JOURNEYS.iter()) {
        assert_eq!(case.id, journey.id);
        assert_eq!(case.intent, journey.intent);
        assert_eq!(case.expected_effects, journey.required_effects);
        assert!(case.prior_authority_preserved_on_failure);
    }
}

#[test]
fn lifecycle_json_rejects_unknown_fields_and_invalid_versions() {
    assert!(serde_json::from_str::<LifecycleRequest>(
        r#"{"intent":"fresh_install","target":null,"prior_authority":null,"authorization":{"allow_host_write":false,"allow_downgrade":false,"expected_installed_sha256":null},"unknown":true}"#
    ).is_err());
    for value in ["1", "1.2", "1.2.3.4", "01.2.3", "a.2.3"] {
        assert_eq!(Version::parse(value), Err(LifecycleError::InvalidVersion));
    }
}
