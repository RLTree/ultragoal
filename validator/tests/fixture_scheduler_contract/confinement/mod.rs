#![cfg(target_os = "macos")]

use crate::fixture_scheduler::*;
use std::collections::{BTreeMap, BTreeSet};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(1);

fn root(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "hul-confinement-{label}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ))
}

fn policy() -> ConfinementPolicy {
    ConfinementPolicy {
        cpu_seconds: 1,
        address_space_bytes: 64 * 1024 * 1024,
        wall_time_millis: 250,
        maximum_file_bytes: 1024 * 1024,
        require_process_group: true,
        network: NetworkIsolation::DenyAll,
    }
}

fn spec(id: &str) -> FixtureSpec {
    FixtureSpec::new_with_confinement(
        id,
        FixtureKind::Positive,
        "resource-confinement",
        BTreeSet::from([
            ResourceKind::File,
            ResourceKind::Process,
            ResourceKind::Port,
        ]),
        ExpectedOutcome::pass(1),
        false,
        policy(),
    )
    .unwrap()
}

fn compile_probe(root: &Path) -> PathBuf {
    std::fs::create_dir_all(root).unwrap();
    let output = root.join("confinement-probe");
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixture_scheduler_contract/confinement/probe.rs");
    assert!(
        Command::new("rustc")
            .arg(source)
            .arg("-o")
            .arg(&output)
            .status()
            .unwrap()
            .success()
    );
    output
}

fn lease<'a>(scheduler: &'a FixtureScheduler, id: &str) -> &'a IsolationLease {
    &scheduler.run(id).unwrap().lease
}

fn run(plan: &ConfinementPlan, probe: &Path, args: &[&str], cwd: &Path) -> std::process::Output {
    plan.command(
        probe,
        &args
            .iter()
            .map(std::ffi::OsString::from)
            .collect::<Vec<_>>(),
        cwd,
        &BTreeMap::new(),
    )
    .unwrap()
    .output()
    .unwrap()
}

#[test]
fn child_observes_cpu_memory_and_file_limits() {
    let root = root("limits");
    let probe = compile_probe(&root.join("build"));
    let mut scheduler = FixtureScheduler::new(root.join("leases"));
    let id = scheduler.schedule([spec("limits")]).unwrap().pop().unwrap();
    let plan = ConfinementPlan::prepare(&policy(), lease(&scheduler, &id).root()).unwrap();
    let address_space_limit = plan.address_space_limit();
    let output = run(&plan, &probe, &["limits"], lease(&scheduler, &id).root());
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        format!("cpu=1 as={address_space_limit} file=1048576")
    );
    scheduler.recover(&id).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn policy_digest_binds_the_child_process_restriction() {
    assert!(policy().digest_fragment().contains("children=deny"));
}

#[test]
fn sandbox_allows_only_lease_writes_and_denies_loopback() {
    let root = root("sandbox");
    let probe = compile_probe(&root.join("build"));
    let mut scheduler = FixtureScheduler::new(root.join("leases"));
    let id = scheduler
        .schedule([spec("sandbox")])
        .unwrap()
        .pop()
        .unwrap();
    let lease_root = lease(&scheduler, &id).root().to_path_buf();
    let plan = ConfinementPlan::prepare(&policy(), &lease_root).unwrap();
    let inside = lease_root.join("file/inside");
    assert!(
        run(
            &plan,
            &probe,
            &["write", inside.to_str().unwrap()],
            &lease_root
        )
        .status
        .success()
    );
    assert!(inside.is_file());
    let outside = root.join("outside-sentinel");
    assert!(
        !run(
            &plan,
            &probe,
            &["write", outside.to_str().unwrap()],
            &lease_root
        )
        .status
        .success()
    );
    assert!(!outside.exists());
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let address = listener.local_addr().unwrap().to_string();
    assert!(
        !run(&plan, &probe, &["connect", &address], &lease_root)
            .status
            .success()
    );
    scheduler.recover(&id).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn sandbox_denies_child_process_creation() {
    let root = root("child-process-denied");
    let probe = compile_probe(&root.join("build"));
    let mut scheduler = FixtureScheduler::new(root.join("leases"));
    let id = scheduler
        .schedule([spec("child-process-denied")])
        .unwrap()
        .pop()
        .unwrap();
    let lease_root = lease(&scheduler, &id).root().to_path_buf();
    let plan = ConfinementPlan::prepare(&policy(), &lease_root).unwrap();
    let output = run(&plan, &probe, &["descendant"], &lease_root);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Operation not permitted"));
    scheduler.recover(&id).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn unavailable_backend_and_limit_substitution_fail_closed() {
    let root = root("fail-closed");
    std::fs::create_dir_all(&root).unwrap();
    assert!(
        ConfinementPlan::prepare_with_backend(&policy(), &root, Path::new("/missing")).is_err()
    );
    let plan = ConfinementPlan::prepare(&policy(), &root).unwrap();
    let substituted_root = root.join("substituted-root");
    std::fs::create_dir(&substituted_root).unwrap();
    assert!(
        plan.command(
            Path::new("/usr/bin/true"),
            &[],
            &substituted_root,
            &BTreeMap::new()
        )
        .is_err()
    );
    let mut forged = spec("substitution");
    forged.confinement.cpu_seconds = 2;
    let mut scheduler = FixtureScheduler::new(root.join("leases"));
    assert!(matches!(
        scheduler.schedule([forged]),
        Err(FixtureScheduleError::Integrity(_))
    ));
    std::fs::remove_dir_all(root).unwrap();
}
