#![cfg(target_os = "macos")]

#[path = "execution_adapter/capture.rs"]
mod capture;
#[path = "execution_adapter/detached_descendant.rs"]
mod detached_descendant;
#[path = "execution_adapter/process_group.rs"]
mod process_group;

use crate::fixture_scheduler::*;
use capture::fixture::FixtureCaptureAdapter;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::PathBuf;
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
    assert_eq!(
        scheduler.execute(&lease, &adapter).unwrap(),
        RunDisposition::Accepted
    );
    assert!(!root.join(lease).exists());
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
    assert_eq!(
        scheduler.execute(&lease, &adapter).unwrap(),
        RunDisposition::CausalFailure
    );
    std::fs::remove_dir_all(root).unwrap();
}
