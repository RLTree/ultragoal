use super::super::scenario::{
    ContainedContender, FaultObservations, Fixture, TerminationFaults, contain_contender,
    git_output, routine_command, run_contender_with_termination_faults, tree,
};
use super::assert_public_busy;
use super::live_child;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::Arc;
use std::time::{Duration, Instant};

const MODE: &str = "HUL_ROUTINE_PUBLIC_CONTENTION_SUPERVISOR";
const CASE: &str = "HUL_ROUTINE_PUBLIC_CONTENTION_CASE";
const ROOT: &str = "HUL_ROUTINE_PUBLIC_CONTENTION_ROOT";
const HOME: &str = "HUL_ROUTINE_PUBLIC_CONTENTION_HOME";
const BINARY: &str = "HUL_ROUTINE_PUBLIC_CONTENTION_BINARY";
const ESCALATION_BOUND: Duration = Duration::from_secs(2);

#[derive(Clone, Copy)]
pub(super) struct SupervisorPlan {
    pub case: &'static str,
    pub execution_bound: Duration,
    pub cleanup_bound: Duration,
    pub primary_kill_refusals: usize,
    pub group_signal_refusals: usize,
    pub reap_status_refusals: usize,
    pub pipe_drain_refusals: usize,
}

#[derive(Debug)]
pub(super) enum SupervisorOutcome {
    Exited(Output),
    TerminatedAndReaped(Output),
    ExitedAtDeadline(Output),
    ReapedWithFailure(&'static str),
}

pub(super) struct SupervisorObservation {
    pub outcome: SupervisorOutcome,
    pub faults: Arc<FaultObservations>,
    pub elapsed: Duration,
}

pub(super) fn run_child_if_requested() -> bool {
    let Some(case) = std::env::var_os(CASE) else {
        return false;
    };
    let case = case.to_string_lossy();
    assert_eq!(std::env::var(MODE).as_deref(), Ok("routine-public-v1"));
    match case.as_ref() {
        "real-public-lock" => real_public_lock_sequence(),
        "hang" => hold_lock_and_wait(false),
        "panic-live-child"
        | "panic-live-child-spawn-refusal"
        | "panic-live-child-early-exit"
        | "panic-live-child-signal-failure"
        | "panic-live-child-wrong-group"
        | "panic-live-child-missing-handshake"
        | "panic-live-child-duplicate-handshake"
        | "panic-live-child-malformed-handshake"
        | "panic-live-child-wrong-binary"
        | "panic-live-child-wrong-sentinel" => live_child::panic_with_live_child(case.as_ref()),
        "ignore-term-descendant" => hold_lock_and_wait(true),
        "pipe-pressure" => pipe_pressure(),
        "exit" => {}
        other => panic!("unknown public contention supervisor case: {other}"),
    }
    true
}

pub(super) fn run_supervisor(
    test_name: &str,
    fixture: &Fixture,
    plan: SupervisorPlan,
) -> SupervisorObservation {
    let (faults, observations) = TerminationFaults::injected(
        plan.primary_kill_refusals,
        plan.group_signal_refusals,
        plan.reap_status_refusals,
        plan.pipe_drain_refusals,
    );
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", test_name, "--nocapture", "--test-threads=1"])
        .env_clear()
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("PATH", "/usr/bin:/bin")
        .env(MODE, "routine-public-v1")
        .env(CASE, plan.case)
        .env(ROOT, &fixture.root)
        .env(HOME, &fixture.home)
        .env(BINARY, fixture.binary_path());
    let started = Instant::now();
    let observed = run_contender_with_termination_faults(
        &mut command,
        plan.execution_bound,
        plan.cleanup_bound,
        faults,
    );
    let outcome = match contain_contender(observed, ESCALATION_BOUND) {
        ContainedContender::Exited(output) => SupervisorOutcome::Exited(output),
        ContainedContender::TerminatedAndReaped(output) => {
            SupervisorOutcome::TerminatedAndReaped(output)
        }
        ContainedContender::ExitedAtDeadline(output) => SupervisorOutcome::ExitedAtDeadline(output),
        ContainedContender::ReapedWithFailure(cause) => SupervisorOutcome::ReapedWithFailure(cause),
    };
    assert!(
        observations.group_is_absent(),
        "supervisor group {} remained after exact child reap",
        observations.process_group()
    );
    SupervisorObservation {
        outcome,
        faults: observations,
        elapsed: started.elapsed(),
    }
}

pub(super) fn finish_fixture(
    fixture: &mut Fixture,
    inspect: impl FnOnce(&Fixture) + std::panic::UnwindSafe,
) {
    let inspected = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| inspect(fixture)));
    fixture.teardown_after_assertions();
    if let Err(payload) = inspected {
        std::panic::resume_unwind(payload);
    }
}

pub(super) fn assert_fixture_lock_released(fixture: &Fixture) {
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .open(fixture.lock_path())
        .unwrap();
    assert_eq!(
        unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
        0
    );
    unlock(lock);
}

fn real_public_lock_sequence() {
    let (root, home, binary) = bound_paths();
    let before_root = tree(&root);
    let before_home = tree(&home);
    let before_status = status(&root);
    let holder = lock(&home);
    let output = public_command(&root, &home, &binary).output().unwrap();
    assert_public_busy(&output);
    assert_eq!(tree(&root), before_root);
    assert_eq!(tree(&home), before_home);
    assert_eq!(status(&root), before_status);
    assert!(!root.join("target").exists());
    assert_eq!(fs::read_dir(authority_root(&home)).unwrap().count(), 0);
    unlock(holder);

    let retry = public_command(&root, &home, &binary).output().unwrap();
    assert_eq!(retry.status.code(), Some(0), "{retry:?}");
    assert!(retry.stderr.is_empty(), "{retry:?}");
    assert!(root.join("target/routine/compile").is_dir());
}

fn hold_lock_and_wait(ignore_term: bool) {
    let (_, home, _) = bound_paths();
    let _holder = lock(&home);
    if ignore_term {
        unsafe { libc::signal(libc::SIGTERM, libc::SIG_IGN) };
        let _descendant = Command::new("/bin/sh")
            .args(["-c", "trap '' TERM; sleep 60 & wait"])
            .spawn()
            .unwrap();
    }
    std::thread::sleep(Duration::from_secs(60));
}

fn pipe_pressure() {
    let (_, home, _) = bound_paths();
    let _holder = lock(&home);
    let bytes = vec![b'x'; 256 * 1024];
    std::io::stdout().write_all(&bytes).unwrap();
    std::io::stdout().flush().unwrap();
    std::thread::sleep(Duration::from_secs(60));
}

pub(super) fn public_command(root: &PathBuf, home: &PathBuf, binary: &PathBuf) -> Command {
    let mut command = routine_command(root, home, binary);
    command.args(["--json", "check", "routine"]);
    command
}

pub(super) fn bound_paths() -> (PathBuf, PathBuf, PathBuf) {
    (path(ROOT), path(HOME), path(BINARY))
}

fn path(name: &str) -> PathBuf {
    PathBuf::from(std::env::var_os(name).unwrap())
}

fn status(root: &PathBuf) -> Vec<u8> {
    git_output(
        root,
        &[
            "--no-optional-locks",
            "status",
            "--porcelain=v2",
            "-z",
            "--untracked-files=all",
        ],
    )
}

fn authority_root(home: &PathBuf) -> PathBuf {
    home.join(".codex/state/harness-ultragoal/routine-public/authority")
}

pub(super) fn lock(home: &PathBuf) -> std::fs::File {
    let holder = OpenOptions::new()
        .read(true)
        .write(true)
        .open(home.join(".codex/state/harness-ultragoal/routine-public/adapter/adapter.lock"))
        .unwrap();
    assert_eq!(
        unsafe { libc::flock(holder.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
        0
    );
    holder
}

fn unlock(holder: std::fs::File) {
    assert_eq!(unsafe { libc::flock(holder.as_raw_fd(), libc::LOCK_UN) }, 0);
    drop(holder);
}
