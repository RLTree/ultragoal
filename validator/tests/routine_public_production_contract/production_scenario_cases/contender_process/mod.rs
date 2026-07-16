mod custody;
mod faults;
mod output;

pub(crate) use custody::ContenderCustody;
pub(crate) use faults::{FaultObservations, TerminationFaults};
use output::CapturedPipes;
use std::os::unix::process::CommandExt;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

const CONTENDER_POLL_CADENCE: Duration = Duration::from_millis(5);
const CONTENDER_CLEANUP_BOUND: Duration = Duration::from_secs(1);

#[derive(Debug)]
#[must_use = "contender outcomes retain process custody until explicitly resolved"]
pub(crate) enum BoundedContender {
    Exited(Output),
    TerminatedAndReaped(Output),
    ExitedAtDeadline(Output),
    ReapedWithFailure(&'static str),
    Unresolved(ContenderCustody),
}

#[derive(Debug)]
pub(crate) enum ContainedContender {
    Exited(Output),
    TerminatedAndReaped(Output),
    ExitedAtDeadline(Output),
    ReapedWithFailure(&'static str),
}

#[derive(Clone, Copy, Debug)]
pub(super) enum Completion {
    Exited,
    Terminated,
    ExitedAtDeadline,
}

pub(crate) fn run_bounded_contender(
    command: &mut Command,
    execution_bound: Duration,
) -> BoundedContender {
    run_with_faults(
        command,
        execution_bound,
        CONTENDER_CLEANUP_BOUND,
        TerminationFaults::default(),
        || {},
    )
}

pub(crate) fn run_contender_with_termination_faults(
    command: &mut Command,
    execution_bound: Duration,
    cleanup_bound: Duration,
    faults: TerminationFaults,
) -> BoundedContender {
    run_with_faults(command, execution_bound, cleanup_bound, faults, || {})
}

pub(crate) fn run_contender_with_termination_faults_after_spawn(
    command: &mut Command,
    execution_bound: Duration,
    cleanup_bound: Duration,
    faults: TerminationFaults,
    after_spawn: impl FnOnce(),
) -> BoundedContender {
    run_with_faults(command, execution_bound, cleanup_bound, faults, after_spawn)
}

pub(crate) fn contain_contender(
    observed: BoundedContender,
    cleanup_bound: Duration,
) -> ContainedContender {
    let unresolved = match into_contained(observed) {
        Ok(contained) => return contained,
        Err(unresolved) => unresolved,
    };
    match into_contained(unresolved.escalate(cleanup_bound)) {
        Ok(contained) => contained,
        Err(unresolved) => {
            eprintln!(
                "supervisor containment could not verify group absence; outer runner must terminate this test process and its reported group: {unresolved:?}"
            );
            std::process::abort()
        }
    }
}

fn into_contained(observed: BoundedContender) -> Result<ContainedContender, ContenderCustody> {
    match observed {
        BoundedContender::Exited(output) => Ok(ContainedContender::Exited(output)),
        BoundedContender::TerminatedAndReaped(output) => {
            Ok(ContainedContender::TerminatedAndReaped(output))
        }
        BoundedContender::ExitedAtDeadline(output) => {
            Ok(ContainedContender::ExitedAtDeadline(output))
        }
        BoundedContender::ReapedWithFailure(cause) => {
            Ok(ContainedContender::ReapedWithFailure(cause))
        }
        BoundedContender::Unresolved(custody) => Err(custody),
    }
}

fn run_with_faults(
    command: &mut Command,
    execution_bound: Duration,
    cleanup_bound: Duration,
    faults: TerminationFaults,
    after_spawn: impl FnOnce(),
) -> BoundedContender {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(_) => return BoundedContender::ReapedWithFailure("contender-spawn-failed"),
    };
    after_spawn();
    let process_group = i32::try_from(child.id()).unwrap();
    faults.observe_process_group(process_group);
    let (pipes, pipe_setup_failure) = CapturedPipes::take(&mut child);
    let custody = ContenderCustody::new(child, pipes, process_group, faults);
    if let Some(cause) = pipe_setup_failure {
        return custody.terminate_primary(cause, cleanup_bound);
    }
    wait_for_contender(custody, execution_bound, cleanup_bound)
}

fn wait_for_contender(
    mut custody: ContenderCustody,
    execution_bound: Duration,
    cleanup_bound: Duration,
) -> BoundedContender {
    let deadline = Instant::now() + execution_bound;
    loop {
        let polled = custody.poll();
        let observed_at = Instant::now();
        match polled {
            custody::Poll::Quiescent if observed_at <= deadline => {
                return custody.into_reaped();
            }
            custody::Poll::Quiescent => {
                return BoundedContender::ReapedWithFailure(
                    "contender-exited-after-execution-deadline",
                );
            }
            custody::Poll::Failed(cause) => {
                return custody.terminate_primary(cause, cleanup_bound);
            }
            custody::Poll::Running => {}
        }
        if observed_at >= deadline {
            return custody.terminate_primary("contender-execution-deadline", cleanup_bound);
        }
        std::thread::sleep(CONTENDER_POLL_CADENCE);
    }
}

#[cfg(test)]
mod controls;
