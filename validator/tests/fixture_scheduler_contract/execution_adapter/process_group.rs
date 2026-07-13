use super::{FixtureCaptureAdapter, root, spec};
use crate::fixture_scheduler::*;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

fn timeout_spec(id: &str, wall_time_millis: u64) -> FixtureSpec {
    FixtureSpec::new_with_confinement(
        id,
        FixtureKind::Positive,
        "confined-capture-timeout",
        BTreeSet::from([
            ResourceKind::File,
            ResourceKind::Env,
            ResourceKind::Process,
            ResourceKind::Port,
        ]),
        ExpectedOutcome::pass(2),
        false,
        ConfinementPolicy {
            cpu_seconds: 5,
            address_space_bytes: 64 * 1024 * 1024,
            wall_time_millis,
            maximum_file_bytes: 1024 * 1024,
            require_process_group: true,
            network: NetworkIsolation::DenyAll,
        },
    )
    .unwrap()
}

pub(super) fn compile_probe(root: &Path) -> PathBuf {
    std::fs::create_dir_all(root).unwrap();
    let output = root.join("confinement-probe");
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixture_scheduler_contract/helpers/confinement_probe.rs");
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

#[derive(Debug)]
pub(super) struct ProcessRecord {
    pub(super) group: i32,
    pub(super) descendants: Vec<i32>,
    pub(super) denied: usize,
}

pub(super) fn read_process_record(path: &Path) -> ProcessRecord {
    let contents = std::fs::read_to_string(path).unwrap();
    let mut lines = contents.lines();
    let group = lines
        .next()
        .unwrap()
        .strip_prefix("group=")
        .unwrap()
        .parse()
        .unwrap();
    let descendants = lines
        .next()
        .unwrap()
        .strip_prefix("descendants=")
        .unwrap()
        .split(',')
        .filter(|value| !value.is_empty())
        .map(|value| value.parse().unwrap())
        .collect();
    let denied = lines
        .next()
        .unwrap()
        .strip_prefix("denied=")
        .unwrap()
        .parse()
        .unwrap();
    ProcessRecord {
        group,
        descendants,
        denied,
    }
}

fn wait_for_readiness(path: &Path) {
    const READY: &[u8] = b"ready\n";
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        match std::fs::symlink_metadata(path) {
            Ok(metadata) => {
                assert!(
                    metadata.file_type().is_file(),
                    "readiness handshake is not a regular file"
                );
                let bytes = std::fs::read(path).expect("read readiness handshake");
                assert!(
                    READY.starts_with(&bytes),
                    "unexpected readiness handshake bytes: {bytes:?}"
                );
                if bytes == READY {
                    return;
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("inspect readiness handshake: {error}"),
        }
        assert!(
            Instant::now() < deadline,
            "fixture never reached the readiness handshake"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn signal_target_absent(target: i32) -> bool {
    if unsafe { libc::kill(target, 0) } == 0 {
        return false;
    }
    let error = std::io::Error::last_os_error();
    assert_eq!(error.raw_os_error(), Some(libc::ESRCH), "{error}");
    true
}

pub(super) fn group_and_descendants_absent(record: &ProcessRecord) -> bool {
    signal_target_absent(-record.group)
        && record
            .descendants
            .iter()
            .all(|pid| signal_target_absent(*pid))
}

pub(super) fn wait_for_natural_test_cleanup(record: &ProcessRecord) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline && !group_and_descendants_absent(record) {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(group_and_descendants_absent(record));
}

#[derive(Clone, Copy)]
enum TerminationTrigger {
    WallTime,
    Interrupt,
    OutputLimit,
}

fn actual_adapter_termination(
    label: &str,
    mode: &str,
    descendants: usize,
    trigger: TerminationTrigger,
) {
    let root = root(label);
    let probe = compile_probe(&root.join("build"));
    let wall_time = if matches!(trigger, TerminationTrigger::WallTime) {
        120
    } else {
        5_000
    };
    let fixture = timeout_spec(label, wall_time);
    let mut scheduler = FixtureScheduler::new(root.join("leases"));
    let lease = scheduler
        .schedule([fixture.clone()])
        .unwrap()
        .pop()
        .unwrap();
    let record_path = scheduler
        .run(&lease)
        .unwrap()
        .lease
        .root()
        .join("file/pids");
    let readiness_path = record_path.with_extension("ready");
    let output_limit = if matches!(trigger, TerminationTrigger::OutputLimit) {
        64
    } else {
        4096
    };
    let adapter = FixtureCaptureAdapter::issue_test_native(
        &fixture,
        probe,
        [
            mode,
            record_path.to_str().unwrap(),
            &descendants.to_string(),
            "1200",
            readiness_path.to_str().unwrap(),
        ]
        .into_iter()
        .map(OsString::from)
        .collect(),
        output_limit,
        Vec::new(),
    )
    .unwrap();
    let outcome = if matches!(trigger, TerminationTrigger::Interrupt) {
        std::thread::scope(|scope| {
            scope.spawn(|| {
                wait_for_readiness(&readiness_path);
                adapter.interrupt();
            });
            let run = scheduler.run(&lease).unwrap();
            adapter.execute(&run.fixture, &run.lease, &run.environment)
        })
    } else {
        let run = scheduler.run(&lease).unwrap();
        adapter.execute(&run.fixture, &run.lease, &run.environment)
    };
    assert_eq!(outcome.unwrap().verdict, OutcomeVerdict::Fail);
    let record = read_process_record(&record_path);
    assert!(record.descendants.is_empty());
    assert_eq!(record.denied, descendants);
    let escaped = !group_and_descendants_absent(&record);
    if escaped {
        wait_for_natural_test_cleanup(&record);
    }
    assert!(
        !escaped,
        "adapter returned before process group {} was absent",
        record.group
    );
    scheduler.recover(&lease).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn interrupt_before_readiness_is_deterministic_without_a_process_record() {
    let root = root("interrupt-before-readiness");
    let probe = compile_probe(&root.join("build"));
    let fixture = timeout_spec("interrupt-before-readiness", 5_000);
    let mut scheduler = FixtureScheduler::new(root.join("leases"));
    let lease = scheduler
        .schedule([fixture.clone()])
        .unwrap()
        .pop()
        .unwrap();
    let record_path = scheduler
        .run(&lease)
        .unwrap()
        .lease
        .root()
        .join("file/pids");
    let readiness_path = record_path.with_extension("ready");
    let adapter = FixtureCaptureAdapter::issue_test_native(
        &fixture,
        probe,
        [
            "delayed-readiness",
            record_path.to_str().unwrap(),
            readiness_path.to_str().unwrap(),
        ]
        .into_iter()
        .map(OsString::from)
        .collect(),
        4096,
        Vec::new(),
    )
    .unwrap();
    adapter.interrupt();
    let run = scheduler.run(&lease).unwrap();
    assert_eq!(
        adapter
            .execute(&run.fixture, &run.lease, &run.environment)
            .unwrap()
            .verdict,
        OutcomeVerdict::Fail
    );
    assert!(!record_path.exists());
    assert!(!readiness_path.exists());
    scheduler.recover(&lease).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn actual_wall_timeout_reaps_early_exiting_direct_child_after_spawn_is_denied() {
    actual_adapter_termination(
        "wall-direct-exit",
        "term-exit-descendants",
        1,
        TerminationTrigger::WallTime,
    );
}

#[test]
fn actual_wall_timeout_reaps_signal_resistant_direct_child_and_denies_multiple_spawns() {
    actual_adapter_termination(
        "wall-all-ignore",
        "all-ignore-descendants",
        3,
        TerminationTrigger::WallTime,
    );
}

#[test]
fn actual_interrupt_and_output_limit_reap_signal_resistant_groups_and_deny_spawns() {
    actual_adapter_termination(
        "interrupt-all-ignore",
        "all-ignore-descendants",
        2,
        TerminationTrigger::Interrupt,
    );
    actual_adapter_termination(
        "output-all-ignore",
        "output-limit-descendants",
        2,
        TerminationTrigger::OutputLimit,
    );
}

#[test]
fn already_exited_process_group_is_absent_after_adapter_return() {
    let root = root("already-exited");
    let probe = compile_probe(&root.join("build"));
    let fixture = spec("already-exited", ExpectedOutcome::pass(2));
    let mut scheduler = FixtureScheduler::new(root.join("leases"));
    let lease = scheduler
        .schedule([fixture.clone()])
        .unwrap()
        .pop()
        .unwrap();
    let record_path = scheduler
        .run(&lease)
        .unwrap()
        .lease
        .root()
        .join("file/pids");
    let adapter = FixtureCaptureAdapter::issue_test_native(
        &fixture,
        probe,
        ["exit-record", record_path.to_str().unwrap()]
            .into_iter()
            .map(OsString::from)
            .collect(),
        4096,
        Vec::new(),
    )
    .unwrap();
    let run = scheduler.run(&lease).unwrap();
    assert_eq!(
        adapter
            .execute(&run.fixture, &run.lease, &run.environment)
            .unwrap()
            .verdict,
        OutcomeVerdict::Pass
    );
    let record = read_process_record(&record_path);
    assert!(group_and_descendants_absent(&record));
    scheduler.recover(&lease).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
