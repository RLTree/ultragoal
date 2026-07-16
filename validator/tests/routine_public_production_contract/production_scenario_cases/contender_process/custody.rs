use super::faults::TerminationFaults;
use super::output::CapturedPipes;
use super::{BoundedContender, CONTENDER_POLL_CADENCE, Completion};
use std::fmt;
use std::process::{Child, ExitStatus};
use std::time::{Duration, Instant};

#[must_use = "unresolved contender custody must be escalated or explicitly retained"]
pub(crate) struct ContenderCustody {
    child: Child,
    pipes: CapturedPipes,
    process_group: i32,
    status: Option<ExitStatus>,
    completion: Completion,
    cause: &'static str,
    pipe_failure: Option<&'static str>,
    faults: TerminationFaults,
}

pub(super) enum Poll {
    Running,
    Quiescent,
    Failed(&'static str),
}

impl ContenderCustody {
    pub(super) fn new(
        child: Child,
        pipes: CapturedPipes,
        process_group: i32,
        faults: TerminationFaults,
    ) -> Self {
        Self {
            child,
            pipes,
            process_group,
            status: None,
            completion: Completion::Exited,
            cause: "contender-unresolved",
            pipe_failure: None,
            faults,
        }
    }

    pub(super) fn poll(&mut self) -> Poll {
        if self.faults.refuse_pipe_drain() {
            let cause = "contender-pipe-drain-injected-refusal";
            self.pipe_failure = Some(cause);
            return Poll::Failed(cause);
        }
        if let Err(cause) = self.pipes.drain_available() {
            self.pipe_failure = Some(cause);
        }
        if self.status.is_none() {
            if self.faults.refuse_reap_status() {
                return Poll::Failed("contender-reap-status-failed");
            }
            match self.child.try_wait() {
                Ok(Some(status)) => self.status = Some(status),
                Ok(None) => {}
                Err(_) => return Poll::Failed("contender-reap-status-failed"),
            }
        }
        match group_exists(self.process_group) {
            Ok(false) if self.status.is_some() && self.pipes.closed() => Poll::Quiescent,
            Ok(_) => Poll::Running,
            Err(cause) => Poll::Failed(cause),
        }
    }

    pub(super) fn into_reaped(self) -> BoundedContender {
        if let Some(cause) = self.pipe_failure {
            return BoundedContender::ReapedWithFailure(cause);
        }
        let output = self
            .pipes
            .into_output(self.status.expect("quiescent custody has a child status"));
        match self.completion {
            Completion::Exited => BoundedContender::Exited(output),
            Completion::Terminated => BoundedContender::TerminatedAndReaped(output),
            Completion::ExitedAtDeadline => BoundedContender::ExitedAtDeadline(output),
        }
    }

    pub(super) fn terminate_primary(
        mut self,
        cause: &'static str,
        cleanup_bound: Duration,
    ) -> BoundedContender {
        self.cause = cause;
        if self.faults.refuse_primary_kill() {
            self.cause = "contender-primary-kill-injected-refusal";
            return BoundedContender::Unresolved(self);
        }
        match self.child.kill() {
            Ok(()) => self.completion = Completion::Terminated,
            Err(_) => match self.child.try_wait() {
                Ok(Some(status)) => {
                    self.status = Some(status);
                    self.completion = Completion::ExitedAtDeadline;
                }
                Ok(None) => {
                    self.cause = "contender-kill-failed-while-live";
                    return BoundedContender::Unresolved(self);
                }
                Err(_) => {
                    self.cause = "contender-reap-status-failed";
                    return BoundedContender::Unresolved(self);
                }
            },
        }
        self.observe_until(Instant::now() + cleanup_bound)
    }

    pub(crate) fn escalate(mut self, cleanup_bound: Duration) -> BoundedContender {
        let deadline = Instant::now() + cleanup_bound;
        let force_at = Instant::now() + cleanup_bound.min(Duration::from_millis(50));
        self.completion = Completion::Terminated;
        let _ = self.signal_group(libc::SIGTERM);
        let mut forced = false;
        loop {
            let polled = self.poll();
            let observed_at = Instant::now();
            match polled {
                Poll::Quiescent if observed_at <= deadline => return self.into_reaped(),
                Poll::Quiescent => {
                    return BoundedContender::ReapedWithFailure(
                        "contender-escalation-completed-after-deadline",
                    );
                }
                Poll::Failed(cause) => self.cause = cause,
                Poll::Running => {}
            }
            if !forced && observed_at >= force_at {
                forced = true;
                let _ = self.signal_group(libc::SIGKILL);
                let _ = self.child.kill();
                self.completion = Completion::Terminated;
            }
            if observed_at >= deadline {
                self.cause = "contender-escalation-deadline-exceeded";
                return BoundedContender::Unresolved(self);
            }
            std::thread::sleep(CONTENDER_POLL_CADENCE);
        }
    }

    pub(crate) fn child_id(&self) -> u32 {
        self.child.id()
    }

    fn observe_until(mut self, deadline: Instant) -> BoundedContender {
        loop {
            let polled = self.poll();
            let observed_at = Instant::now();
            match polled {
                Poll::Quiescent if observed_at <= deadline => return self.into_reaped(),
                Poll::Quiescent => {
                    return BoundedContender::ReapedWithFailure(
                        "contender-cleanup-completed-after-deadline",
                    );
                }
                Poll::Failed(cause) => self.cause = cause,
                Poll::Running => {}
            }
            if observed_at >= deadline {
                return BoundedContender::Unresolved(self);
            }
            std::thread::sleep(CONTENDER_POLL_CADENCE);
        }
    }

    fn signal_group(&mut self, signal: i32) -> Result<(), &'static str> {
        if self.faults.refuse_group_signal() {
            self.cause = "contender-group-signal-injected-refusal";
            return Err(self.cause);
        }
        let target = self
            .process_group
            .checked_neg()
            .ok_or("contender-group-invalid")?;
        if unsafe { libc::kill(target, signal) } == 0
            || std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
        {
            Ok(())
        } else {
            self.cause = "contender-group-signal-failed";
            Err(self.cause)
        }
    }
}

impl fmt::Debug for ContenderCustody {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ContenderCustody")
            .field("child_id", &self.child.id())
            .field("process_group", &self.process_group)
            .field("child_reaped", &self.status.is_some())
            .field("cause", &self.cause)
            .finish()
    }
}

pub(super) fn group_exists(group: i32) -> Result<bool, &'static str> {
    let target = group.checked_neg().ok_or("contender-group-invalid")?;
    if unsafe { libc::kill(target, 0) } == 0 {
        return Ok(true);
    }
    match std::io::Error::last_os_error().raw_os_error() {
        Some(libc::ESRCH) => Ok(false),
        Some(libc::EPERM) => Ok(true),
        _ => Err("contender-group-probe-failed"),
    }
}
