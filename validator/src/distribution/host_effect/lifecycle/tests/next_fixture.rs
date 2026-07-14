static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

fn d(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

struct Fixture {
    root: PathBuf,
    home: PathBuf,
    project: PathBuf,
    executable: PathBuf,
    alternate_executable: PathBuf,
    package: PackageIdentity,
    host: HostCapabilityDeclaration,
    journey: JourneyBinding,
    lifecycle: AcceptedLifecyclePlan,
    scope: AcceptedHostScope,
    plan: HostCommandPlan,
    target: ObservedTargetIdentity,
}

impl Fixture {
    fn new(seed: char) -> Self {
        let root = std::env::temp_dir().join(format!(
            "hul-supported-host-lifecycle-063-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let home = root.join("home");
        let project = root.join("project");
        fs::create_dir(&home).unwrap();
        fs::create_dir(&project).unwrap();
        let executable = root.join("codex");
        let alternate_executable = root.join("codex-other");
        fs::write(&executable, b"descriptor-execution-fixture-v1").unwrap();
        fs::write(&alternate_executable, b"descriptor-execution-fixture-v1").unwrap();
        #[cfg(unix)]
        {
            fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
            fs::set_permissions(&alternate_executable, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let source = SourceIdentity::new(
            d(seed),
            d(next_hex(seed)),
            "harness-ultragoal".to_owned(),
            "0.0.11".to_owned(),
            d('c'),
            d('d'),
        )
        .unwrap();
        let package = PackageIdentity::new(source, d('e'), d('f')).unwrap();
        let host = HostCapabilityDeclaration::isolated(
            &home,
            &project,
            "host-lifecycle-063-v1",
            Some(&executable),
        )
        .unwrap();
        let journey = JourneyBinding::new(package.clone(), &host, "local-marketplace").unwrap();
        let lifecycle = lifecycle(&package);
        let scope = AcceptedHostScope::personal(&journey, "local-marketplace".to_owned()).unwrap();
        let plan = HostCommandPlan::personal_install(&package, "local-marketplace").unwrap();
        let object =
            HostObjectIdentity::from_metadata(&fs::symlink_metadata(&project).unwrap()).unwrap();
        let target = ObservedTargetIdentity::new(&scope, 7, object).unwrap();
        Self {
            root,
            home,
            project,
            executable,
            alternate_executable,
            package,
            host,
            journey,
            lifecycle,
            scope,
            plan,
            target,
        }
    }

    fn pin(&self) -> PinnedHostExecutable {
        PinnedHostExecutable::pin(&self.executable).unwrap()
    }

    fn pin_alternate(&self) -> PinnedHostExecutable {
        PinnedHostExecutable::pin(&self.alternate_executable).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn acceptance<'a>(
    fixture: &'a Fixture,
    executable: &'a PinnedHostExecutable,
    expected_head: HostEffectLedgerHead,
) -> HostEffectAcceptanceRequest<'a> {
    HostEffectAcceptanceRequest {
        package: fixture.package.clone(),
        journey: fixture.journey.clone(),
        host: fixture.host.clone(),
        lifecycle: fixture.lifecycle.clone(),
        scope: fixture.scope.clone(),
        plan: &fixture.plan,
        executable,
        expected_target: fixture.target.clone(),
        expected_head,
    }
}

fn preparation<'a>(
    accepted: &'a AcceptedHostEffect,
    custody: &'a mut RootPlanCustody,
    executable: PinnedHostExecutable,
    target: &'a mut dyn HostTargetObserver,
    clock: &'a mut dyn RootTrustedClock,
    adapter: &'a mut dyn DescriptorExecutionAdapter,
) -> HostEffectPreparationRequest<'a> {
    HostEffectPreparationRequest {
        accepted,
        custody,
        executable,
        target,
        clock,
        adapter,
    }
}

fn lifecycle(package: &PackageIdentity) -> AcceptedLifecyclePlan {
    let before = AcceptedHostState::new(0, None, false).unwrap();
    let after = AcceptedHostState::new(1, Some(package.clone()), false).unwrap();
    let rollback = AcceptedHostState::new(0, None, false).unwrap();
    AcceptedLifecyclePlan::new(
        AcceptedLifecycleOperation::FreshInstall,
        before,
        after,
        rollback,
        AcceptedRollbackPolicy::RemoveOnlyNewTarget,
        AcceptedReconciliationPolicy::ExactPostStateAndSeparateHostLayers,
    )
    .unwrap()
}

fn next_hex(value: char) -> char {
    match value {
        '0'..='8' => char::from_u32(value as u32 + 1).unwrap(),
        '9' => 'a',
        'a'..='e' => char::from_u32(value as u32 + 1).unwrap(),
        _ => '1',
    }
}

struct RecordingLedger {
    inner: Mutex<LedgerState>,
    head_calls: AtomicUsize,
    reserve_calls: AtomicUsize,
    transition_calls: AtomicUsize,
}

struct LedgerState {
    head: HostEffectLedgerHead,
    records: BTreeMap<String, HostEffectLedgerRecord>,
}

impl RecordingLedger {
    fn new(generation: u64, head_sha256: String) -> Self {
        Self {
            inner: Mutex::new(LedgerState {
                head: HostEffectLedgerHead::new(generation, head_sha256).unwrap(),
                records: BTreeMap::new(),
            }),
            head_calls: AtomicUsize::new(0),
            reserve_calls: AtomicUsize::new(0),
            transition_calls: AtomicUsize::new(0),
        }
    }

    fn observed_head(&self) -> HostEffectLedgerHead {
        self.inner.lock().unwrap().head.clone()
    }

    fn writes(&self) -> usize {
        self.reserve_calls.load(Ordering::Relaxed) + self.transition_calls.load(Ordering::Relaxed)
    }
}
