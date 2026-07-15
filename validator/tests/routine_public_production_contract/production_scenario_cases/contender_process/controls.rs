use super::faults::TerminationFaults;
use super::output::MAX_CAPTURE_BYTES;
use super::*;

#[test]
fn normal_exit_captures_bounded_streams() {
    let mut command = Command::new("/bin/sh");
    command.args(["-c", "printf stdout; printf stderr >&2"]);
    match resolve_control(run_bounded_contender(&mut command, Duration::from_secs(1))) {
        ContainedContender::Exited(output) => {
            assert_eq!(output.stdout, b"stdout");
            assert_eq!(output.stderr, b"stderr");
        }
        other => panic!("normal contender did not exit: {other:?}"),
    }
}

#[test]
fn pipe_pressure_is_drained_before_verified_termination() {
    let mut command = Command::new("/usr/bin/yes");
    let started = Instant::now();
    match resolve_control(run_bounded_contender(
        &mut command,
        Duration::from_millis(100),
    )) {
        ContainedContender::TerminatedAndReaped(output) => {
            assert!(started.elapsed() < Duration::from_secs(1));
            assert_eq!(output.stderr, b"");
            assert_eq!(output.stdout.len(), MAX_CAPTURE_BYTES);
        }
        other => panic!("pipe-pressure contender was not reaped: {other:?}"),
    }
}

#[test]
fn primary_kill_refusal_retains_child_until_group_escalation_reaps_it() {
    let (faults, observed) = TerminationFaults::injected(1, 0, 0, 0);
    let mut command = Command::new("/bin/sleep");
    command.arg("60");
    let custody = match run_with_faults(
        &mut command,
        Duration::from_millis(20),
        Duration::from_millis(100),
        faults,
    ) {
        BoundedContender::Unresolved(custody) => custody,
        other => panic!("primary kill refusal lost custody: {other:?}"),
    };
    assert_eq!(observed.primary_kill_refusals(), 1);
    let child = custody.child_id();
    match resolve_control(BoundedContender::Unresolved(custody)) {
        ContainedContender::TerminatedAndReaped(_) => {}
        other => panic!("group escalation did not reap child {child}: {other:?}"),
    }
}

#[test]
fn reap_status_refusals_are_observed_before_bounded_reap() {
    let (faults, observed) = TerminationFaults::injected(0, 0, 2, 0);
    let mut command = Command::new("/bin/sleep");
    command.arg("60");
    match resolve_control(run_with_faults(
        &mut command,
        Duration::from_millis(20),
        Duration::from_secs(1),
        faults,
    )) {
        ContainedContender::TerminatedAndReaped(_) => {}
        other => panic!("reap-status recovery did not finish: {other:?}"),
    }
    assert_eq!(observed.reap_status_refusals(), 2);
}

#[test]
fn late_reap_retains_the_same_child_until_a_later_bounded_escalation() {
    let (faults, observed) = TerminationFaults::injected(0, 0, 20, 0);
    let mut command = Command::new("/bin/sleep");
    command.arg("60");
    let custody = match run_with_faults(
        &mut command,
        Duration::from_millis(20),
        Duration::from_millis(40),
        faults,
    ) {
        BoundedContender::Unresolved(custody) => custody,
        other => panic!("late-reap fault did not retain custody: {other:?}"),
    };
    let child = custody.child_id();
    match resolve_control(BoundedContender::Unresolved(custody)) {
        ContainedContender::TerminatedAndReaped(_) => {}
        other => panic!("late child {child} was not reaped by escalation: {other:?}"),
    }
    assert_eq!(observed.reap_status_refusals(), 20);
}

#[test]
fn group_signal_refusal_falls_through_to_forceful_group_reap() {
    let (faults, observed) = TerminationFaults::injected(1, 1, 0, 0);
    let mut command = Command::new("/bin/sleep");
    command.arg("60");
    let custody = match run_with_faults(
        &mut command,
        Duration::from_millis(20),
        Duration::from_millis(100),
        faults,
    ) {
        BoundedContender::Unresolved(custody) => custody,
        other => panic!("primary refusal did not retain custody: {other:?}"),
    };
    match resolve_control(BoundedContender::Unresolved(custody)) {
        ContainedContender::TerminatedAndReaped(_) => {}
        other => panic!("forceful group fallback did not reap: {other:?}"),
    }
    assert_eq!(observed.group_signal_refusals(), 1);
}

#[test]
fn descendant_holding_pipes_is_killed_with_the_owned_group() {
    let mut command = Command::new("/bin/sh");
    command.args(["-c", "sleep 60 & wait"]);
    let first = run_bounded_contender(&mut command, Duration::from_millis(50));
    let custody = match first {
        BoundedContender::Unresolved(custody) => custody,
        other => panic!("descendant custody was not preserved: {other:?}"),
    };
    match resolve_control(BoundedContender::Unresolved(custody)) {
        ContainedContender::TerminatedAndReaped(_) => {}
        other => panic!("owned process group was not reaped: {other:?}"),
    }
}

#[test]
fn pipe_drain_failure_cannot_hide_an_unreaped_process() {
    let (faults, observed) = TerminationFaults::injected(0, 0, 0, 1);
    let mut command = Command::new("/bin/sleep");
    command.arg("60");
    match resolve_control(run_with_faults(
        &mut command,
        Duration::from_secs(1),
        Duration::from_secs(1),
        faults,
    )) {
        ContainedContender::ReapedWithFailure(cause) => {
            assert_eq!(cause, "contender-pipe-drain-injected-refusal");
        }
        other => panic!("pipe failure returned before verified reap: {other:?}"),
    }
    assert_eq!(observed.pipe_drain_refusals(), 1);
}

#[test]
fn exit_at_deadline_race_returns_only_a_reaped_state() {
    let mut command = Command::new("/usr/bin/true");
    let observed = resolve_control(run_bounded_contender(&mut command, Duration::ZERO));
    match observed {
        ContainedContender::Exited(output)
        | ContainedContender::ExitedAtDeadline(output)
        | ContainedContender::TerminatedAndReaped(output) => {
            assert!(output.status.success() || output.status.code().is_none());
        }
        ContainedContender::ReapedWithFailure(cause) => {
            panic!("exit/deadline race lost output after reap: {cause}")
        }
    }
}

fn resolve_control(observed: BoundedContender) -> ContainedContender {
    contain_contender(observed, Duration::from_secs(1))
}
