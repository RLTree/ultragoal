#[derive(Default)]
struct MemoryInstall(Option<Vec<u8>>);

impl InstallEffects for MemoryInstall {
    fn read_installed(
        &mut self,
        _: &str,
        _: usize,
    ) -> Result<Option<Vec<u8>>, crate::distribution::EffectFailure> {
        Ok(self.0.clone())
    }

    fn compare_exchange_installed(
        &mut self,
        _: &str,
        expected: &ExpectedPrior,
        replacement: Option<&[u8]>,
    ) -> Result<bool, crate::distribution::EffectFailure> {
        let matches = match expected {
            ExpectedPrior::Absent => self.0.is_none(),
            ExpectedPrior::ExactDigest(expected) => {
                self.0
                    .as_deref()
                    .map(crate::distribution_fixture::digest)
                    .as_deref()
                    == Some(expected)
            }
        };
        if matches {
            self.0 = replacement.map(<[u8]>::to_vec);
        }
        Ok(matches)
    }
}

#[cfg(unix)]
#[test]
fn confined_install_authority_issues_installed_and_runtime_surfaces() {
    let (fixture, package, installed, executable, host, binding) =
        runtime_fixture("install-authority-positive");
    let mut install_effects = ScopedInstall::new(fixture.confined());
    let installed_surface = SurfaceIdentity::from_verified_install(
        installed.snapshot(),
        &binding,
        &mut install_effects,
    )
    .unwrap();
    assert_eq!(
        installed_surface.journey_binding_sha256(),
        Some(binding.binding_sha256())
    );
    let mut runtime_install_effects = ScopedInstall::new(fixture.confined());
    let plan = RuntimeProbePlan::from_installed_package(InstalledPackageRuntimeProbeRequest {
        binding,
        host: &host,
        install: installed.snapshot(),
        effects: &mut runtime_install_effects,
        package: &package,
        program: &executable,
        argv: valid_args(),
        timeout: Duration::from_secs(10),
    })
    .unwrap();
    let (_, runtime_surface) = plan.execute_bound().unwrap();
    assert_eq!(
        runtime_surface.surface(),
        crate::distribution::IdentitySurface::Runtime
    );
}

#[cfg(unix)]
#[test]
fn memory_install_snapshot_cannot_issue_installed_or_runtime_authority() {
    let fixture = JourneyFixture::new("install-memory-forgery");
    let package = fixture.build("packages/runtime.hugpkg");
    let plan = InstallPlan::new(
        package.context_id().into(),
        package.candidate_id().into(),
        InstallScope::PersonalFixture,
        "plugins/harness-ultragoal.hugpkg".into(),
        package.package_sha256().into(),
        ExpectedPrior::Absent,
    )
    .unwrap();
    let mut memory = MemoryInstall::default();
    let mut installed = install(&plan, &package, &mut memory).unwrap();
    let executable = installed_program(&fixture.root);
    let host = HostCapabilityDeclaration::isolated(
        &fixture.root,
        &fixture.project,
        "isolated-host-v1",
        Some(&executable),
    )
    .unwrap();
    let binding =
        JourneyBinding::new(package.identity().clone(), &host, "local-harness-plugins").unwrap();

    assert_eq!(
        installed.bind_journey(&binding).unwrap_err().id(),
        ErrorId::ProvenanceMismatch
    );
    assert_eq!(
        SurfaceIdentity::from_verified_install(installed.snapshot(), &binding, &mut memory)
            .unwrap_err()
            .id(),
        ErrorId::ProvenanceMismatch
    );
    assert_eq!(
        RuntimeProbePlan::from_installed_package(InstalledPackageRuntimeProbeRequest {
            binding,
            host: &host,
            install: installed.snapshot(),
            effects: &mut memory,
            package: &package,
            program: &executable,
            argv: valid_args(),
            timeout: Duration::from_secs(10),
        })
        .unwrap_err()
        .id(),
        ErrorId::ProvenanceMismatch
    );
}

#[cfg(unix)]
#[test]
fn cross_root_install_snapshot_cannot_mint_other_journey_authority() {
    let (fixture_a, package_a, installed_a, _executable_a, _host_a, _binding_a) =
        runtime_fixture("install-root-a");
    let fixture_b = JourneyFixture::new("install-root-b");
    let package_b = fixture_b.build("packages/runtime.hugpkg");
    assert_eq!(package_a.identity(), package_b.identity());
    let executable_b = installed_program(&fixture_b.root);
    let host_b = HostCapabilityDeclaration::isolated(
        &fixture_b.root,
        &fixture_b.project,
        "isolated-host-v1",
        Some(&executable_b),
    )
    .unwrap();
    let binding_b = JourneyBinding::new(
        package_b.identity().clone(),
        &host_b,
        "local-harness-plugins",
    )
    .unwrap();
    let mut effects_b = ScopedInstall::new(fixture_b.confined());

    assert_eq!(
        SurfaceIdentity::from_verified_install(installed_a.snapshot(), &binding_b, &mut effects_b)
            .unwrap_err()
            .id(),
        ErrorId::ProvenanceMismatch
    );
    let mut runtime_effects_b = ScopedInstall::new(fixture_b.confined());
    assert_eq!(
        RuntimeProbePlan::from_installed_package(InstalledPackageRuntimeProbeRequest {
            binding: binding_b,
            host: &host_b,
            install: installed_a.snapshot(),
            effects: &mut runtime_effects_b,
            package: &package_b,
            program: &executable_b,
            argv: valid_args(),
            timeout: Duration::from_secs(10),
        })
        .unwrap_err()
        .id(),
        ErrorId::ProvenanceMismatch
    );
    drop(fixture_a);
}

#[cfg(unix)]
#[test]
fn missing_or_replaced_installed_object_invalidates_install_authority() {
    let (fixture, _package, installed, _executable, _host, binding) =
        runtime_fixture("install-object-revalidation");
    let installed_path = fixture.root.join("plugins/harness-ultragoal.hugpkg");
    replace_with_same_bytes(&installed_path, "same-bytes-installed-object");
    let mut effects = ScopedInstall::new(fixture.confined());
    assert_eq!(
        SurfaceIdentity::from_verified_install(installed.snapshot(), &binding, &mut effects)
            .unwrap_err()
            .id(),
        ErrorId::ObjectChanged
    );

    std::fs::remove_file(&installed_path).unwrap();
    let mut missing_effects = ScopedInstall::new(fixture.confined());
    assert_eq!(
        SurfaceIdentity::from_verified_install(
            installed.snapshot(),
            &binding,
            &mut missing_effects,
        )
        .unwrap_err()
        .id(),
        ErrorId::ObjectUnavailable
    );
}

#[cfg(unix)]
#[test]
fn wrong_target_and_mutate_restore_install_snapshots_fail_closed() {
    let fixture = JourneyFixture::new("install-wrong-target");
    let package = fixture.build("packages/runtime.hugpkg");
    let wrong_target_plan = InstallPlan::new(
        package.context_id().into(),
        package.candidate_id().into(),
        InstallScope::PersonalFixture,
        "plugins/other.hugpkg".into(),
        package.package_sha256().into(),
        ExpectedPrior::Absent,
    )
    .unwrap();
    let mut wrong_target = install(
        &wrong_target_plan,
        &package,
        &mut ScopedInstall::new(fixture.confined()),
    )
    .unwrap();
    let executable = installed_program(&fixture.root);
    let host = HostCapabilityDeclaration::isolated(
        &fixture.root,
        &fixture.project,
        "isolated-host-v1",
        Some(&executable),
    )
    .unwrap();
    let binding =
        JourneyBinding::new(package.identity().clone(), &host, "local-harness-plugins").unwrap();
    assert_eq!(
        wrong_target.bind_journey(&binding).unwrap_err().id(),
        ErrorId::ProvenanceMismatch
    );

    let (fixture, _package, installed, _executable, _host, binding) =
        runtime_fixture("install-mutate-restore");
    let installed_path = fixture.root.join("plugins/harness-ultragoal.hugpkg");
    let original = std::fs::read(&installed_path).unwrap();
    std::fs::write(&installed_path, b"temporary mutation").unwrap();
    std::fs::write(&installed_path, original).unwrap();
    let mut effects = ScopedInstall::new(fixture.confined());
    assert_eq!(
        SurfaceIdentity::from_verified_install(installed.snapshot(), &binding, &mut effects)
            .unwrap_err()
            .id(),
        ErrorId::ObjectChanged
    );
}
