use super::super::scenario::{
    ContainedContender, FaultObservations, Fixture, TerminationFaults, contain_contender,
    git_output, routine_command, run_contender_with_termination_faults_after_spawn, tree,
};
use super::assert_public_busy;
use super::invocation_capability::{self, CapabilityMode, Invocation};
use super::live_child;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::Arc;
use std::time::{Duration, Instant};

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
    pub capability: CapabilityMode,
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

pub(super) fn run_child_if_requested(expected_test: &str) -> bool {
    let invocation = match invocation_capability::consume(expected_test) {
        Ok(None) => return false,
        Ok(Some(invocation)) => invocation,
        Err(error) => panic!("routine public helper capability refused: {error}"),
    };
    match invocation.case.as_str() {
        "capability-probe" => println!("routine-public-capability-v1: helper-selected"),
        "capability-replay" => {
            assert!(
                invocation_capability::consume(expected_test).is_err(),
                "a consumed helper capability remained usable"
            );
            println!("routine-public-capability-v1: replay-refused");
        }
        "real-public-lock" => real_public_lock_sequence(&invocation),
        "hang" => hold_lock_and_wait(&invocation, false),
        "panic-live-child"
        | "panic-live-child-spawn-refusal"
        | "panic-live-child-early-exit"
        | "panic-live-child-signal-failure"
        | "panic-live-child-wrong-group"
        | "panic-live-child-missing-handshake"
        | "panic-live-child-duplicate-handshake"
        | "panic-live-child-malformed-handshake"
        | "panic-live-child-wrong-binary"
        | "panic-live-child-wrong-sentinel" => live_child::panic_with_live_child(&invocation),
        "ignore-term-descendant" => hold_lock_and_wait(&invocation, true),
        "pipe-pressure" => pipe_pressure(&invocation),
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
        .env("PATH", "/usr/bin:/bin");
    let capability =
        invocation_capability::issue(&mut command, fixture, test_name, plan.case, plan.capability);
    let started = Instant::now();
    let observed = run_contender_with_termination_faults_after_spawn(
        &mut command,
        plan.execution_bound,
        plan.cleanup_bound,
        faults,
        || capability.close_after_spawn(),
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

fn real_public_lock_sequence(invocation: &Invocation) {
    let before_root = tree(&invocation.root);
    let before_home = tree(&invocation.home);
    let before_status = status(&invocation.root);
    let holder = lock(&invocation.home);
    let output = public_command(&invocation.root, &invocation.home, &invocation.binary)
        .output()
        .unwrap();
    assert_public_busy(&output);
    assert_eq!(tree(&invocation.root), before_root);
    assert_eq!(tree(&invocation.home), before_home);
    assert_eq!(status(&invocation.root), before_status);
    assert!(!invocation.root.join("target").exists());
    assert!(
        !invocation
            .root
            .join("validation_artifacts/observability/spool/successor-events.jsonl")
            .exists()
    );
    assert_eq!(
        fs::read_dir(authority_root(&invocation.home))
            .unwrap()
            .count(),
        0
    );
    unlock(holder);

    let retry = public_command(&invocation.root, &invocation.home, &invocation.binary)
        .output()
        .unwrap();
    assert_eq!(retry.status.code(), Some(0), "{retry:?}");
    assert!(retry.stderr.is_empty(), "{retry:?}");
    assert!(invocation.root.join("target/routine/compile").is_dir());
}

fn hold_lock_and_wait(invocation: &Invocation, ignore_term: bool) {
    let _holder = lock(&invocation.home);
    if ignore_term {
        unsafe { libc::signal(libc::SIGTERM, libc::SIG_IGN) };
        let _descendant = Command::new("/bin/sh")
            .args(["-c", "trap '' TERM; sleep 60 & wait"])
            .spawn()
            .unwrap();
    }
    std::thread::sleep(Duration::from_secs(60));
}

fn pipe_pressure(invocation: &Invocation) {
    let _holder = lock(&invocation.home);
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
