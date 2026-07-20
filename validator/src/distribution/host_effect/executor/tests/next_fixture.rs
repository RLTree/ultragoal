use crate::plugin_product::lifecycle::{
    plan, HostLifecycleBinding, HostLifecycleCustody, HostLifecycleExpectedObservations,
    LifecycleAuthorization, LifecycleIntent, LifecycleRequest, PackageAuthority, Version,
};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    roots: Vec<PathBuf>,
    target_root: PathBuf,
    ledger_root: PathBuf,
    executable: PathBuf,
    home: PathBuf,
    project: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let suffix = format!(
            "{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        );
        let target_root = PathBuf::from(format!("/private/tmp/hul-supported-host-effect-{suffix}"));
        let ledger_root = PathBuf::from(format!("/private/tmp/hul-executor-ledger-{suffix}"));
        let support_root = PathBuf::from(format!("/private/tmp/hul-executor-support-{suffix}"));
        create_mode(&target_root, 0o700);
        create_mode(&support_root, 0o700);
        let home = support_root.join("home");
        let project = support_root.join("project");
        create_mode(&home, 0o700);
        create_mode(&project, 0o700);
        let executable = support_root.join("codex-fixture");
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&executable)
            .unwrap();
        file.write_all(b"#!/bin/sh\nexit 0\n").unwrap();
        file.sync_all().unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        Self {
            roots: vec![target_root.clone(), ledger_root.clone(), support_root],
            target_root,
            ledger_root,
            executable,
            home,
            project,
        }
    }

    fn package(&self) -> PackageIdentity {
        let source = SourceIdentity::new(
            digest('1'),
            digest('2'),
            "harness-ultragoal".to_owned(),
            "0.0.11".to_owned(),
            digest('3'),
            digest('4'),
        )
        .unwrap();
        PackageIdentity::new(source, digest('5'), digest('6')).unwrap()
    }

    fn scope_and_target(
        &self,
    ) -> (
        AcceptedHostScope,
        ConfinedHostEffectTarget,
        crate::distribution::host_effect::lifecycle::ObservedTargetIdentity,
    ) {
        let package = self.package();
        let host = HostCapabilityDeclaration::isolated(
            &self.home,
            &self.project,
            "fixture-host",
            Some(&self.executable),
        )
        .unwrap();
        let journey = JourneyBinding::new(package, &host, "fixture-marketplace").unwrap();
        let scope =
            AcceptedHostScope::personal(&journey, "fixture-marketplace".to_owned()).unwrap();
        let (target, identity) =
            ConfinedHostEffectTarget::bind(&self.target_root, scope.clone(), 1).unwrap();
        (scope, target, identity)
    }
}

fn lifecycle_custody(fixture: &Fixture, command_plan: &HostCommandPlan) -> HostLifecycleCustody {
    let package = fixture.package();
    let authority = PackageAuthority {
        version: Version::parse(package.source().version()).unwrap(),
        package_sha256: package.archive_sha256().to_owned(),
        inventory_sha256: package.tree_sha256().to_owned(),
        candidate_id: package.source().candidate_id().to_owned(),
    };
    let lifecycle = plan(
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
            package,
            command_plan.clone(),
            digest('1'),
            digest('2'),
            HostLifecycleExpectedObservations {
                installed_sha256: digest('3'),
                cache_sha256: digest('4'),
                registry_sha256: digest('5'),
                discovery_sha256: digest('6'),
                runtime_sha256: digest('7'),
                command_count: command_plan.commands().len(),
            },
        )
        .unwrap(),
    )
    .unwrap()
}

impl Drop for Fixture {
    fn drop(&mut self) {
        for root in self.roots.iter().rev() {
            if root.starts_with("/private/tmp/hul-") {
                let _ = fs::remove_dir_all(root);
            }
        }
    }
}

fn create_mode(path: &Path, mode: u32) {
    fs::DirBuilder::new().mode(mode).create(path).unwrap();
}

fn digest(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

fn permit_binding(
    fixture: &Fixture,
    ledger: &FileHostEffectLedger,
    target: &crate::distribution::host_effect::lifecycle::ObservedTargetIdentity,
    candidate_digest_byte: char,
) -> HostEffectPermitBinding {
    let package = fixture.package();
    let plan = HostCommandPlan::personal_install(&package, "fixture-marketplace").unwrap();
    let executable = PinnedHostExecutable::pin(&fixture.executable).unwrap();
    let head = ledger.head().unwrap();
    HostEffectPermitBinding {
        context_id: package.source().context_id().to_owned(),
        candidate_id: digest(candidate_digest_byte),
        package_identity_sha256: digest('3'),
        journey_binding_sha256: digest('4'),
        session_issuance_sha256: digest('5'),
        lifecycle_plan_sha256: digest('6'),
        lifecycle_intent: "install".to_owned(),
        expected_pre_state_sha256: digest('7'),
        expected_post_state_sha256: digest('8'),
        rollback_policy_sha256: digest('9'),
        reconciliation_policy_sha256: digest('a'),
        host_scope_sha256: digest('b'),
        host_capability_sha256: digest('c'),
        required_capabilities_sha256: digest('d'),
        external_request_sha256: digest('e'),
        command_plan_sha256: plan.plan_sha256().to_owned(),
        argv_sha256: digest('f'),
        executable_identity_sha256: executable.identity().binding_sha256().unwrap(),
        target_identity_sha256: target.target_sha256().to_owned(),
        target_generation: target.generation(),
        issued_at_unix_ms: 1_000,
        expires_at_unix_ms: 200_000,
        expected_head_sha256: head.head_sha256().to_owned(),
        lifecycle_record: None,
        lifecycle_record_sha256: None,
        decision: HostEffectDecision::Authorize,
    }
}

fn authorized_effect(
    fixture: &Fixture,
    ledger: &FileHostEffectLedger,
    target: &crate::distribution::host_effect::lifecycle::ObservedTargetIdentity,
) -> AuthorizedHostEffect {
    let package = fixture.package();
    let plan = HostCommandPlan::personal_install(&package, "fixture-marketplace").unwrap();
    let executable = PinnedHostExecutable::pin(&fixture.executable).unwrap();
    let binding = permit_binding(fixture, ledger, target, '2');
    let authority =
        HostEffectAuthority::generate("fixture-root".to_owned(), "fixture-ledger".to_owned())
            .unwrap();
    let (permit, reservation) = authority.issue(binding).unwrap();
    let reserved = ledger.reserve(reservation).unwrap();
    let in_flight = ledger
        .transition(
            HostEffectTransition::new(
                permit.permit_id().to_owned(),
                HostEffectState::Reserved,
                HostEffectState::InFlight,
                reserved.current_head().clone(),
                None,
            )
            .unwrap(),
        )
        .unwrap();
    AuthorizedHostEffect::new(permit, in_flight, executable, plan).unwrap()
}

struct TestClock {
    sequence: u64,
}

impl RootTrustedClock for TestClock {
    fn sample(
        &mut self,
    ) -> Result<
        TrustedTimeSample,
        crate::distribution::host_effect::lifecycle::SupportedHostLifecycleError,
    > {
        self.sequence += 1;
        TrustedTimeSample::new(
            "executor-test-clock".to_owned(),
            1,
            self.sequence,
            10_000 + self.sequence,
        )
    }
}

struct FailingClock;
