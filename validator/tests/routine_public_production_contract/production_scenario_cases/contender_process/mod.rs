mod custody;
mod faults;
mod output;

pub(crate) use custody::ContenderCustody;
use faults::TerminationFaults;
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
    )
}

fn run_with_faults(
    command: &mut Command,
    execution_bound: Duration,
    cleanup_bound: Duration,
    faults: TerminationFaults,
) -> BoundedContender {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    let mut child = command.spawn().unwrap();
    let process_group = i32::try_from(child.id()).unwrap();
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
