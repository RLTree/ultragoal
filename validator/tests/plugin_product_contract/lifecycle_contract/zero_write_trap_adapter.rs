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
