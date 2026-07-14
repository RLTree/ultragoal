use crate::distribution::{
    DistributionErrorId as ErrorId, HostCapabilityDeclaration, JourneyBinding, RuntimeObservation,
    RuntimeProbePlan, RuntimeVerdict, execute_runtime_probe,
};
use crate::distribution_fixture::Fixture;
use crate::package_journey_fixture::JourneyFixture;
use serde_json::json;
use std::path::PathBuf;
use std::time::Duration;

pub fn program() -> PathBuf {
    std::env::current_exe().unwrap()
}

pub fn valid_args() -> Vec<String> {
    vec![
        "--exact".into(),
        "runtime_session::runtime_probe_child".into(),
        "--nocapture".into(),
        "--test-threads=1".into(),
    ]
}

fn stale_args() -> Vec<String> {
    vec![
        "--exact".into(),
        "runtime_session::runtime_probe_child_stale".into(),
        "--nocapture".into(),
        "--test-threads=1".into(),
    ]
}

fn slow_args() -> Vec<String> {
    vec![
        "--exact".into(),
        "runtime_session::runtime_probe_child_slow".into(),
        "--nocapture".into(),
        "--test-threads=1".into(),
    ]
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
    let allowed = [
        "HUL_CONTEXT_ID",
        "HUL_CANDIDATE_ID",
        "HUL_PLUGIN_ID",
        "HUL_VERSION",
        "HUL_PACKAGE_SHA256",
        "HUL_TREE_SHA256",
        "HUL_HOME_ID",
        "HUL_PROJECT_ID",
        "HUL_HOST_ID",
        "HUL_CAPABILITY_SHA256",
        "HUL_BINDING_SHA256",
        "HUL_EXECUTABLE_SHA256",
        "HUL_SESSION_NONCE",
    ];
    assert!(std::env::vars().all(|(key, _)| allowed.contains(&key.as_str())));
    let value = json!({
        "schema":"harness-ultragoal.runtime-probe.v1",
        "context_id":env("HUL_CONTEXT_ID"),
        "candidate_id":env("HUL_CANDIDATE_ID"),
        "plugin_id":env("HUL_PLUGIN_ID"),
        "version":if stale { "0.0.10".into() } else { env("HUL_VERSION") },
        "package_sha256":env("HUL_PACKAGE_SHA256"),
        "installed_tree_sha256":env("HUL_TREE_SHA256"),
        "home_id":env("HUL_HOME_ID"),
        "project_id":env("HUL_PROJECT_ID"),
        "host_id":env("HUL_HOST_ID"),
        "capability_sha256":env("HUL_CAPABILITY_SHA256"),
        "binding_sha256":env("HUL_BINDING_SHA256"),
        "executable_sha256":env("HUL_EXECUTABLE_SHA256"),
        "session_nonce":env("HUL_SESSION_NONCE"),
    });
    println!("HUL_RUNTIME_OBSERVATION={value}");
}

fn env(name: &str) -> String {
    std::env::var(name).unwrap()
}

#[test]
fn stale_subprocess_receipt_and_dormant_report_cannot_become_runtime_proof() {
    let fixture = JourneyFixture::new("runtime-stale");
    let package = fixture.build("package/runtime.hugpkg");
    let executable = program();
    let host = HostCapabilityDeclaration::isolated(
        &fixture.root,
        &fixture.project,
        "isolated-host-v1",
        Some(&executable),
    )
    .unwrap();
    let binding =
        JourneyBinding::new(package.identity().clone(), &host, "local-harness-plugins").unwrap();
    let stale = RuntimeProbePlan::new(
        binding.clone(),
        &host,
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
    let original = program();
    let copied = fixture.root.join("runtime-probe-bin");
    std::fs::copy(&original, &copied).unwrap();
    std::fs::set_permissions(&copied, std::fs::Permissions::from_mode(0o755)).unwrap();
    let host = HostCapabilityDeclaration::isolated(
        &fixture.root,
        &fixture.project,
        "isolated-host-v1",
        Some(&copied),
    )
    .unwrap();
    let binding =
        JourneyBinding::new(package.identity().clone(), &host, "local-harness-plugins").unwrap();
    let plan = RuntimeProbePlan::new(
        binding,
        &host,
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
