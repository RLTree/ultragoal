use crate::plugin_product::lifecycle::{
    HostLifecycleBinding, HostLifecycleCustody, HostLifecycleExpectedObservations,
    HostLifecycleRecord, LifecycleAuthorization, LifecycleIntent, LifecycleRequest,
    PackageAuthority, Version,
};

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
    selected_fixture: SelectedCodexExecutableTestFixture,
    alternate_fixture: SelectedCodexExecutableTestFixture,
    package: PackageIdentity,
    host: HostCapabilityDeclaration,
    journey: JourneyBinding,
    lifecycle: AcceptedLifecyclePlan,
    scope: AcceptedHostScope,
    plan: HostCommandPlan,
    projection: HostCommandPlanProjection,
    lifecycle_record: HostLifecycleRecord,
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
        let selected_fixture =
            selected_test_fixture("lifecycle-primary", b"descriptor-execution-fixture-v1");
        let alternate_fixture =
            selected_test_fixture("lifecycle-alternate", b"descriptor-execution-fixture-v1");
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
        let host = selected_fixture
            .host_capability(&home, &project, "host-lifecycle-063-v1")
            .unwrap();
        let journey = JourneyBinding::new(package.clone(), &host, "local-marketplace").unwrap();
        let lifecycle = lifecycle(&package);
        let scope = AcceptedHostScope::personal(&journey, "local-marketplace".to_owned()).unwrap();
        let plan = HostCommandPlan::personal_install(&package, "local-marketplace").unwrap();
        let projection = plan.projection().unwrap();
        let lifecycle_record = lifecycle_custody_for(&package, &plan)
            .pre_effect_record()
            .clone();
        let object =
            HostObjectIdentity::from_metadata(&fs::symlink_metadata(&project).unwrap()).unwrap();
        let target = ObservedTargetIdentity::new(&scope, 7, object).unwrap();
        Self {
            root,
            home,
            project,
            selected_fixture,
            alternate_fixture,
            package,
            host,
            journey,
            lifecycle,
            scope,
            plan,
            projection,
            lifecycle_record,
            target,
        }
    }

    fn pin(&self) -> SelectedCodexExecutable {
        self.selected_fixture.selected().unwrap()
    }

    fn pin_alternate(&self) -> SelectedCodexExecutable {
        self.alternate_fixture.selected().unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn acceptance<'a>(
    fixture: &'a Fixture,
    executable: &'a SelectedCodexExecutable,
    expected_head: HostEffectLedgerHead,
) -> HostEffectAcceptanceRequest<'a> {
    HostEffectAcceptanceRequest {
        package: fixture.package.clone(),
        journey: fixture.journey.clone(),
        host: fixture.host.clone(),
        lifecycle: fixture.lifecycle.clone(),
        scope: fixture.scope.clone(),
        plan: &fixture.projection,
        executable,
        expected_target: fixture.target.clone(),
        expected_head,
        lifecycle_record: &fixture.lifecycle_record,
    }
}

fn acceptance_with_custody<'a>(
    fixture: &'a Fixture,
    executable: &'a SelectedCodexExecutable,
    expected_head: HostEffectLedgerHead,
    custody: &'a HostLifecycleCustody,
) -> HostEffectAcceptanceRequest<'a> {
    let mut request = acceptance(fixture, executable, expected_head);
    request.lifecycle_record = custody.pre_effect_record();
    request
}

fn preparation<'a>(
    accepted: &'a AcceptedHostEffect,
    custody: &'a mut HostLifecycleCustody,
    executable: SelectedCodexExecutable,
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

fn lifecycle_custody(fixture: &Fixture) -> HostLifecycleCustody {
    lifecycle_custody_for(&fixture.package, &fixture.plan)
}

fn lifecycle_custody_for(
    package: &PackageIdentity,
    command_plan: &HostCommandPlan,
) -> HostLifecycleCustody {
    let authority = PackageAuthority {
        version: Version::parse(package.source().version()).unwrap(),
        package_sha256: package.archive_sha256().to_owned(),
        inventory_sha256: package.tree_sha256().to_owned(),
        candidate_id: package.source().candidate_id().to_owned(),
    };
    let lifecycle = crate::plugin_product::lifecycle::plan(
        &crate::plugin_product::lifecycle::LifecycleState::default(),
        &LifecycleRequest {
            intent: LifecycleIntent::FreshInstall,
            target: Some(authority),
            prior_authority: None,
            authorization: LifecycleAuthorization {
                allow_host_write: true,
                allow_downgrade: false,
                expected_installed_sha256: None,
            },
        },
    )
    .unwrap();
    HostLifecycleCustody::take(
        lifecycle,
        HostLifecycleBinding::new(
            package.clone(),
            command_plan.clone(),
            d('1'),
            d('2'),
            HostLifecycleExpectedObservations {
                installed_sha256: d('3'),
                cache_sha256: d('4'),
                registry_sha256: d('5'),
                discovery_sha256: d('6'),
                runtime_sha256: d('7'),
                command_count: command_plan.commands().len(),
            },
        )
        .unwrap(),
    )
    .unwrap()
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
