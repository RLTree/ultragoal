use crate::distribution::{
    DistributionErrorId as ErrorId, ExpectedPrior, HostCapabilityDeclaration, InstallEffects,
    InstallPlan, InstallScope, InstallTransaction, JourneyBinding, PackageSnapshot,
    RuntimeObservation, RuntimeProbePlan, RuntimeVerdict, ScopedInstall, SurfaceIdentity,
    execute_runtime_probe, install,
};
use crate::distribution_fixture::Fixture;
use crate::package_journey_fixture::{JourneyFixture, runtime_probe_bytes};
use serde_json::json;
use std::path::PathBuf;
use std::time::Duration;

pub fn program() -> PathBuf {
    PathBuf::from("runtime/runtime-probe-bin")
}

pub fn installed_program(root: &std::path::Path) -> PathBuf {
    let target = root.join("runtime/runtime-probe-bin");
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::write(&target, runtime_probe_bytes()).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    target
}

pub fn valid_args() -> Vec<String> {
    vec!["valid".into()]
}

fn stale_args() -> Vec<String> {
    vec!["stale".into()]
}

fn slow_args() -> Vec<String> {
    vec!["slow".into()]
}

#[test]
fn runtime_probe_child() {
    if std::env::var("HUL_SESSION_NONCE").is_err() {
        return;
    }
    emit(false);
}

#[test]
fn runtime_probe_child_stale() {
    if std::env::var("HUL_SESSION_NONCE").is_err() {
        return;
    }
    emit(true);
}

#[test]
fn runtime_probe_child_slow() {
    if std::env::var("HUL_SESSION_NONCE").is_err() {
        return;
    }
    std::thread::sleep(Duration::from_millis(150));
    emit(false);
}

fn emit(stale: bool) {
    let allowed = ["HUL_SESSION_NONCE"];
    assert!(std::env::vars().all(|(key, _)| allowed.contains(&key.as_str())));
    let value = json!({
        "schema":"harness-ultragoal.runtime-probe.v1",
        "session_nonce":if stale { "stale".into() } else { env("HUL_SESSION_NONCE") },
    });
    println!("HUL_RUNTIME_OBSERVATION={value}");
}

fn env(name: &str) -> String {
    std::env::var(name).unwrap()
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
fn stale_subprocess_receipt_and_dormant_report_cannot_become_runtime_proof() {
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
    let stale = RuntimeProbePlan::from_installed_package(
        binding.clone(),
        &host,
        installed.snapshot(),
        &mut install_effects,
        &package,
        &executable,
        stale_args(),
        Duration::from_secs(10),
    )
    .unwrap();
    assert_eq!(
        execute_runtime_probe(&stale).unwrap_err().id(),
        ErrorId::ProvenanceMismatch
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
    let plan = RuntimeProbePlan::from_installed_package(
        binding,
        &host,
        installed.snapshot(),
        &mut install_effects,
        &package,
        &copied,
        slow_args(),
        Duration::from_secs(10),
    )
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

include!("runtime_session/inode_swap.rs");

include!("runtime_session/install_authority.rs");
