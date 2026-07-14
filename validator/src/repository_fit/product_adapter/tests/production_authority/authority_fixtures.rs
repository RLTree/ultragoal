use super::*;

impl<'a> RevalidationStore<'a> {
    pub(crate) fn new(inner: &'a TestStore, accepted_calls: u64) -> Self {
        Self {
            inner,
            calls: AtomicU64::new(0),
            accepted_calls,
        }
    }
}

impl RepositoryFitAuthorityStore for RevalidationStore<'_> {
    fn protected_root(&self) -> &Path {
        &self.inner.root
    }

    fn store_id(&self) -> &str {
        &self.inner.id
    }

    fn revalidate_protected_root(&self) -> bool {
        self.calls.fetch_add(1, Ordering::SeqCst) < self.accepted_calls
    }
}

pub(crate) struct TestClock {
    pub(crate) ticks: Mutex<VecDeque<u64>>,
}

impl TestClock {
    pub(crate) fn new(ticks: impl IntoIterator<Item = u64>) -> Self {
        Self {
            ticks: Mutex::new(ticks.into_iter().collect()),
        }
    }
}

impl RepositoryFitTrustedClock for TestClock {
    fn trusted_tick(&self) -> Result<u64, FitAdapterError> {
        self.ticks.lock().unwrap().pop_front().ok_or_else(|| {
            crate::repository_fit::product_adapter::adapter_error(
                AdapterErrorId::ApplyPermitExpired,
            )
        })
    }
}

pub(crate) fn nonce(label: &str) -> RepositoryFitApplyNonce {
    RepositoryFitApplyNonce::new(digest(label.as_bytes()).into_bytes()).unwrap()
}

pub(crate) fn execute(
    fixture: &Fixture,
    context: &LiveContext,
    prepared: PreparedFitApply,
    label: &str,
) -> crate::repository_fit::product_adapter::authority::RepositoryFitProductionOutcome {
    execute_with_ticks(fixture, context, prepared, label, [10, 11, 12])
}

pub(crate) fn execute_with_ticks(
    fixture: &Fixture,
    context: &LiveContext,
    prepared: PreparedFitApply,
    label: &str,
    ticks: impl IntoIterator<Item = u64>,
) -> crate::repository_fit::product_adapter::authority::RepositoryFitProductionOutcome {
    let recovery_intent = prepare_recovery_intent(context, &prepared).unwrap();
    execute_prepared_apply(
        context,
        prepared,
        recovery_intent,
        &TestClock::new(ticks),
        &fixture.store,
        nonce(label),
    )
}

pub(crate) fn git(root: &Path, arguments: &[&str]) {
    let status = Command::new("/usr/bin/git")
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .args(arguments)
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success());
}

pub(crate) fn fixed_digest(character: char) -> String {
    format!("sha256:{}", character.to_string().repeat(64))
}

pub(crate) fn reservation<'a>(character: char) -> ReservationRequest<'a> {
    let values = Box::leak(Box::new([
        fixed_digest(character),
        fixed_digest(char::from_u32(character as u32 + 1).unwrap()),
        fixed_digest(char::from_u32(character as u32 + 2).unwrap()),
        fixed_digest(char::from_u32(character as u32 + 3).unwrap()),
        fixed_digest(char::from_u32(character as u32 + 4).unwrap()),
        fixed_digest(char::from_u32(character as u32 + 5).unwrap()),
    ]));
    let recovery = Box::leak(Box::new(RecoveryTargetSpec {
        request_id: fixed_digest('f'),
        root_binding: fixed_digest('e'),
        ancestors: managed_ancestor_contract_for_ledger_test(),
        rows: vec![RecoveryTargetRow {
            path: "AGENTS.md".to_owned(),
            pre_sha256: None,
            pre_mode: None,
            post_sha256: fixed_digest('d'),
            post_mode: 0o644,
        }],
    }));
    values[5] = digest(&canonical_recovery_intent_bytes(recovery).unwrap());
    ReservationRequest {
        binding_sha256: &values[0],
        semantic_effect_id: &values[1],
        target_scope_id: &values[2],
        permit_id: &values[3],
        nonce_sha256: &values[4],
        recovery_intent_sha256: &values[5],
        issued_tick: 10,
        expires_tick: 20,
        recovery,
    }
}

pub(crate) fn authority_scenario_command(
    fixture: &Fixture,
    role: &str,
    nonce_label: &str,
    result_path: &Path,
    ready_path: &Path,
    release_path: &Path,
) -> Command {
    authority_scenario_command_at_root(
        fixture,
        &fixture.root,
        role,
        nonce_label,
        result_path,
        ready_path,
        release_path,
    )
}

pub(crate) fn authority_scenario_command_at_root(
    fixture: &Fixture,
    repository_root: &Path,
    role: &str,
    nonce_label: &str,
    result_path: &Path,
    ready_path: &Path,
    release_path: &Path,
) -> Command {
    let intent_path = recovery_intent_path(fixture, nonce_label);
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([AUTHORITY_SCENARIO_HELPER, "--nocapture"])
        .env("HUL_FIT_AUTHORITY_SCENARIO", role)
        .env("HUL_FIT_AUTHORITY_REPO", repository_root)
        .env("HUL_FIT_AUTHORITY_STORE", &fixture.store.root)
        .env("HUL_FIT_AUTHORITY_STORE_ID", &fixture.store.id)
        .env("HUL_FIT_AUTHORITY_NONCE_LABEL", nonce_label)
        .env("HUL_FIT_AUTHORITY_INTENT", intent_path)
        .env("HUL_FIT_AUTHORITY_RESULT", result_path)
        .env("HUL_FIT_AUTHORITY_READY", ready_path)
        .env("HUL_FIT_AUTHORITY_RELEASE", release_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

pub(crate) fn recovery_intent_path(fixture: &Fixture, nonce_label: &str) -> PathBuf {
    fixture.container.join(format!(
        "{}-recovery-intent.json",
        nonce_label.replace(|character: char| !character.is_ascii_alphanumeric(), "-")
    ))
}
