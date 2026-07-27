use crate::distribution::{
    DistributionErrorId as ErrorId, ExpectedPrior, HostCapabilityDeclaration, InstallEffects,
    InstallPlan, InstallScope, InstallTransaction, InstalledPackageRuntimeProbeRequest,
    JourneyBinding, PackageSnapshot, RuntimeObservation, RuntimeProbePlan, RuntimeVerdict,
    ScopedInstall, SurfaceIdentity, execute_runtime_probe, install, uninstall,
};
use crate::distribution_fixture::Fixture;
use crate::package_journey_fixture::{JourneyFixture, runtime_probe_bytes};
use std::path::PathBuf;
use std::time::Duration;

pub fn installed_program(root: &std::path::Path) -> PathBuf {
    let target = root.join("plugins/harness-ultragoal/runtime/ultragoal");
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::write(&target, runtime_probe_bytes()).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    target
}

fn install_for_runtime(fixture: &JourneyFixture, package: &PackageSnapshot) -> InstallTransaction {
    let plan = InstallPlan::new(
        package.context_id().into(),
        package.candidate_id().into(),
        InstallScope::PersonalFixture,
        "plugins/harness-ultragoal.hugpkg".into(),
        package.package_sha256().into(),
        ExpectedPrior::Absent,
    )
    .unwrap();
    install(&plan, package, &mut ScopedInstall::new(fixture.confined())).unwrap()
}

#[test]
fn fixed_help_execution_and_dormant_report_stay_separate_runtime_evidence() {
    let fixture = JourneyFixture::new("runtime-stale");
    let package = fixture.build("package/runtime.hugpkg");
    let mut installed = install_for_runtime(&fixture, &package);
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
    installed.bind_journey(&binding).unwrap();
    let mut install_effects = ScopedInstall::new(fixture.confined());
    let current = RuntimeProbePlan::from_installed_package(InstalledPackageRuntimeProbeRequest {
        binding: binding.clone(),
        host: &host,
        install: installed.snapshot(),
        effects: &mut install_effects,
        package: &package,
        program: &executable,
        timeout: Duration::from_secs(10),
    })
    .unwrap();
    assert_eq!(
        execute_runtime_probe(&current).unwrap().runtime_verdict(),
        RuntimeVerdict::Executed
    );

    let report_fixture = Fixture::complete("dormant-runtime");
    let report =
        crate::distribution::verify(&report_fixture.root, &report_fixture.bytes()).unwrap();
    let dormant = RuntimeObservation::from_report(&report);
    assert_eq!(dormant.runtime_verdict(), RuntimeVerdict::DefinitionOnly);
    assert!(!dormant.is_current_execution());
}

#[test]
fn unavailable_runtime_lowers_only_runtime_surface() {
    let fixture = JourneyFixture::new("runtime-unavailable");
    let package = fixture.build("package/runtime.hugpkg");
    let host = HostCapabilityDeclaration::unavailable_codex_app(
        &fixture.root,
        &fixture.project,
        "codex-app-api-unavailable",
    )
    .unwrap();
    let binding =
        JourneyBinding::new(package.identity().clone(), &host, "local-harness-plugins").unwrap();
    let observation = RuntimeObservation::unavailable(&binding, &host).unwrap();
    assert_eq!(observation.runtime_verdict(), RuntimeVerdict::Unsupported);
    assert!(!observation.is_current_execution());
}

#[cfg(unix)]
#[test]
fn executable_substitution_during_probe_fails_final_revalidation() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = JourneyFixture::new("runtime-executable-race");
    let package = fixture.build("packages/runtime.hugpkg");
    let mut installed = install_for_runtime(&fixture, &package);
    let copied = installed_program(&fixture.root);
    let host = HostCapabilityDeclaration::isolated(
        &fixture.root,
        &fixture.project,
        "isolated-host-v1",
        Some(&copied),
    )
    .unwrap();
    let binding =
        JourneyBinding::new(package.identity().clone(), &host, "local-harness-plugins").unwrap();
    installed.bind_journey(&binding).unwrap();
    let mut install_effects = ScopedInstall::new(fixture.confined());
    let plan = RuntimeProbePlan::from_installed_package(InstalledPackageRuntimeProbeRequest {
        binding,
        host: &host,
        install: installed.snapshot(),
        effects: &mut install_effects,
        package: &package,
        program: &copied,
        timeout: Duration::from_secs(10),
    })
    .unwrap();
    let replacement = fixture.root.join("runtime-probe-replacement");
    let copied_for_thread = copied.clone();
    let mut bytes = std::fs::read(&copied).unwrap();
    let last = bytes.last_mut().unwrap();
    *last ^= 1;
    let race = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(40));
        std::fs::write(&replacement, bytes).unwrap();
        std::fs::set_permissions(&replacement, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::rename(replacement, copied_for_thread).unwrap();
    });
    let result = execute_runtime_probe(&plan);
    race.join().unwrap();
    assert_eq!(result.unwrap_err().id(), ErrorId::ObjectChanged);
}

include!("inode_swap.rs");

include!("install_authority.rs");

include!("install_staleness.rs");
