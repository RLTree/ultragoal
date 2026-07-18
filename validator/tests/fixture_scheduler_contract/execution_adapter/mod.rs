#![cfg(target_os = "macos")]

#[path = "detached_descendant.rs"]
mod detached_descendant;
#[path = "process_group/mod.rs"]
mod process_group;

use crate::cli::capture::fixture::FixtureCaptureAdapter;
use crate::fixture_scheduler::*;
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(1);

fn root(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "hul-fixture-adapter-{label}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ))
}

fn spec(id: &str, expected: ExpectedOutcome) -> FixtureSpec {
    FixtureSpec::new(
        id,
        if expected.verdict == OutcomeVerdict::Pass {
            FixtureKind::Positive
        } else {
            FixtureKind::Negative
        },
        "confined-capture-execution",
        BTreeSet::from([
            ResourceKind::File,
            ResourceKind::Env,
            ResourceKind::Process,
            ResourceKind::Port,
        ]),
        expected,
        false,
    )
    .unwrap()
}

struct RecordingExecutor<'a> {
    inner: &'a FixtureCaptureAdapter,
    observed: RefCell<Option<ObservedOutcome>>,
}

impl<'a> RecordingExecutor<'a> {
    fn new(inner: &'a FixtureCaptureAdapter) -> Self {
        Self {
            inner,
            observed: RefCell::new(None),
        }
    }

    fn observed(&self) -> Option<ObservedOutcome> {
        self.observed.borrow().clone()
    }
}

impl FixtureExecutor for RecordingExecutor<'_> {
    fn execute(
        &self,
        fixture: &FixtureSpec,
        lease: &IsolationLease,
        environment: &std::collections::BTreeMap<String, String>,
    ) -> Result<ObservedOutcome, FixtureScheduleError> {
        let observed = FixtureExecutor::execute(self.inner, fixture, lease, environment)?;
        self.observed.replace(Some(observed.clone()));
        Ok(observed)
    }
}

fn assert_exact_retained_run(
    scheduler: &FixtureScheduler,
    repository_root: &Path,
    lease_id: &str,
    fixture_id: &str,
) {
    assert_eq!(scheduler.active_count(), 1);
    let run = scheduler.run(lease_id).unwrap();
    let lease_root = repository_root.join(lease_id);
    assert_eq!(run.fixture.id, fixture_id);
    assert_eq!(run.lease.id(), lease_id);
    assert_eq!(run.lease.fixture_id(), fixture_id);
    assert_eq!(run.lease.root(), lease_root);
    assert_eq!(run.lease.disposition(), &LeaseDisposition::RecoveryRequired);
    assert!(!run.is_active());
    assert_eq!(run.lease.bindings().len(), 4);
    assert_eq!(run.environment.len(), 4);
    assert_eq!(
        std::fs::read_to_string(lease_root.join(".fixture-lease")).unwrap(),
        format!("fixture={fixture_id}\n")
    );

    for kind in [
        ResourceKind::File,
        ResourceKind::Env,
        ResourceKind::Process,
        ResourceKind::Port,
    ] {
        let binding = run
            .lease
            .bindings()
            .iter()
            .find(|binding| binding.kind == kind)
            .unwrap();
        let environment_key = format!("HUL_FIXTURE_{}", kind.label().to_ascii_uppercase());
        assert_eq!(run.environment.get(&environment_key), Some(&binding.key));

        let namespace = lease_root.join(kind.label());
        assert!(namespace.is_dir(), "missing namespace {namespace:?}");
        if kind == ResourceKind::Port {
            assert_eq!(
                std::fs::read_to_string(namespace.join("reservation")).unwrap(),
                binding.key
            );
        } else {
            assert_eq!(binding.key, namespace.to_str().unwrap());
        }
    }
}

fn execute_and_assert_retained(
    scheduler: &mut FixtureScheduler,
    repository_root: &Path,
    lease_id: &str,
    fixture_id: &str,
    adapter: &FixtureCaptureAdapter,
    expected_observation: ObservedOutcome,
) {
    let executor = RecordingExecutor::new(adapter);
    assert_eq!(
        scheduler.execute(lease_id, &executor).unwrap(),
        RunDisposition::CleanupFailure
    );
    assert_eq!(executor.observed(), Some(expected_observation));
    assert_exact_retained_run(scheduler, repository_root, lease_id, fixture_id);

    assert_eq!(
        scheduler.recover(lease_id).unwrap(),
        RunDisposition::CleanupFailure
    );
    assert_exact_retained_run(scheduler, repository_root, lease_id, fixture_id);
}

#[test]
fn pinned_capture_adapter_executes_inside_the_confinement_plan() {
    let root = root("pass");
    let fixture = spec("capture-pass", ExpectedOutcome::pass(2));
    let adapter = FixtureCaptureAdapter::issue(
        &fixture,
        "/usr/bin/true".into(),
        Vec::<OsString>::new(),
        4096,
        Vec::new(),
    )
    .unwrap();
    let mut scheduler = FixtureScheduler::new(&root);
    let lease = scheduler.schedule([fixture]).unwrap().pop().unwrap();
    execute_and_assert_retained(
        &mut scheduler,
        &root,
        &lease,
        "capture-pass",
        &adapter,
        ObservedOutcome::pass(2),
    );
    drop(scheduler);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn public_scheduled_execution_derives_the_outcome_from_the_child() {
    let root = root("public-execution");
    let fixture = spec("public-execution", ExpectedOutcome::pass(2));
    let mut scheduler = FixtureScheduler::new(&root);
    let lease = scheduler.schedule([fixture]).unwrap().pop().unwrap();
    assert_eq!(
        crate::capture::execute_scheduled_fixture(
            &mut scheduler,
            &lease,
            "/usr/bin/true".into(),
            Vec::new(),
            4096,
            Vec::new(),
        )
        .unwrap(),
        RunDisposition::CleanupFailure
    );
    assert_exact_retained_run(&scheduler, &root, &lease, "public-execution");
    drop(scheduler);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn nonzero_capture_adapter_preserves_the_bound_causal_control() {
    let root = root("failure");
    let fixture = spec(
        "capture-failure",
        ExpectedOutcome::causal_failure("intentional", 1),
    );
    let adapter = FixtureCaptureAdapter::issue(
        &fixture,
        "/usr/bin/false".into(),
        Vec::<OsString>::new(),
        4096,
        Vec::new(),
    )
    .unwrap();
    let mut scheduler = FixtureScheduler::new(&root);
    let lease = scheduler.schedule([fixture]).unwrap().pop().unwrap();
    execute_and_assert_retained(
        &mut scheduler,
        &root,
        &lease,
        "capture-failure",
        &adapter,
        ObservedOutcome::failure("intentional", 1),
    );
    drop(scheduler);
    std::fs::remove_dir_all(root).unwrap();
}
