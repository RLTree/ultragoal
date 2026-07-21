use super::darwin::DarwinSuspendedProcess;
use super::darwin_capture::{
    DarwinCaptureHandles, discard_after_cleanup, join_capture, start_capture,
};
use super::darwin_cleanup::{cleanup_process, group_exists};
use std::io;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
pub(crate) struct DarwinProcessPolicy {
    pub(crate) timeout: Duration,
    pub(crate) stdout_limit: usize,
    pub(crate) stderr_limit: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DarwinProcessTermination {
    Exited(i32),
    Signaled(i32),
    DescendantSurvived,
}

pub(crate) struct DarwinProcessResult {
    pub(crate) termination: DarwinProcessTermination,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr_sha256: String,
    pub(crate) output_byte_length: u64,
}

pub(crate) trait DarwinProcessHooks {
    fn before_resume(&mut self) -> io::Result<()> {
        Ok(())
    }
    fn before_wait(&mut self) -> io::Result<()> {
        Ok(())
    }
    fn before_stdout_join(&mut self) -> io::Result<()> {
        Ok(())
    }
    fn before_stderr_join(&mut self) -> io::Result<()> {
        Ok(())
    }
    fn before_stdin_join(&mut self) -> io::Result<()> {
        Ok(())
    }
    fn before_cleanup(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DarwinProcessFailure {
    Resume,
    Wait,
    Cancelled,
    Timeout,
    OutputOverflow,
    Capture,
    Cleanup,
}

struct DarwinProcessSession {
    process: DarwinSuspendedProcess,
    captures: Option<DarwinCaptureHandles>,
    reaped: bool,
}

impl DarwinProcessSession {
    fn new(process: DarwinSuspendedProcess) -> Self {
        Self {
            process,
            captures: None,
            reaped: false,
        }
    }

    fn abort(&mut self, hooks: &mut dyn DarwinProcessHooks) -> bool {
        let hook_ok = catch_unwind(AssertUnwindSafe(|| hooks.before_cleanup()))
            .ok()
            .is_some_and(|result| result.is_ok());
        let process_ok =
            cleanup_process(self.process.pid, self.process.group(), self.reaped).is_ok();
        let capture_ok = match self.captures.take() {
            Some(handles) if process_ok => discard_after_cleanup(handles).is_ok(),
            Some(_) => false,
            None => true,
        };
        hook_ok && process_ok && capture_ok
    }
}

pub(crate) fn execute_process(
    process: DarwinSuspendedProcess,
    input: &[u8],
    policy: DarwinProcessPolicy,
    cancelled: impl Fn() -> bool,
    hooks: &mut dyn DarwinProcessHooks,
) -> Result<DarwinProcessResult, DarwinProcessFailure> {
    let mut session = DarwinProcessSession::new(process);
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        execute_inner(&mut session, input, policy, cancelled, hooks)
    }));
    match outcome {
        Ok(Ok(result)) => {
            session.process.mark_settled();
            Ok(result)
        }
        Ok(Err(failure)) => {
            if session.abort(hooks) {
                Err(failure)
            } else {
                Err(DarwinProcessFailure::Cleanup)
            }
        }
        Err(payload) => {
            let _ = session.abort(hooks);
            resume_unwind(payload)
        }
    }
}

fn execute_inner(
    session: &mut DarwinProcessSession,
    input: &[u8],
    policy: DarwinProcessPolicy,
    cancelled: impl Fn() -> bool,
    hooks: &mut dyn DarwinProcessHooks,
) -> Result<DarwinProcessResult, DarwinProcessFailure> {
    if cancelled() {
        return Err(DarwinProcessFailure::Cancelled);
    }
    hooks
        .before_resume()
        .map_err(|_| DarwinProcessFailure::Resume)?;
    // SAFETY: the spawned process owns a group whose identity is its pid.
    if unsafe { libc::kill(-session.process.group(), libc::SIGCONT) } != 0 {
        return Err(DarwinProcessFailure::Resume);
    }
    let (stdin, stdout, stderr) = session
        .process
        .take_pipes()
        .map_err(|_| DarwinProcessFailure::Capture)?;
    let captures = start_capture(
        stdin,
        stdout,
        stderr,
        input,
        policy.stdout_limit,
        policy.stderr_limit,
    );
    session.captures = Some(match captures {
        Ok(handles) => handles,
        Err(failure) => {
            session.captures = Some(failure.handles);
            return Err(DarwinProcessFailure::Capture);
        }
    });
    let deadline = Instant::now() + policy.timeout;
    let mut status = 0;
    loop {
        hooks
            .before_wait()
            .map_err(|_| DarwinProcessFailure::Wait)?;
        if cancelled() {
            return Err(DarwinProcessFailure::Cancelled);
        }
        if session
            .captures
            .as_ref()
            .is_some_and(DarwinCaptureHandles::overflowed)
        {
            return Err(DarwinProcessFailure::OutputOverflow);
        }
        if Instant::now() >= deadline {
            return Err(DarwinProcessFailure::Timeout);
        }
        // SAFETY: process.pid is the child returned by posix_spawn and status is writable.
        let waited = unsafe { libc::waitpid(session.process.pid, &mut status, libc::WNOHANG) };
        if waited == session.process.pid {
            session.reaped = true;
            break;
        }
        if waited < 0 && io::Error::last_os_error().raw_os_error() != Some(libc::EINTR) {
            return Err(DarwinProcessFailure::Wait);
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    hooks
        .before_stdout_join()
        .map_err(|_| DarwinProcessFailure::Capture)?;
    hooks
        .before_stderr_join()
        .map_err(|_| DarwinProcessFailure::Capture)?;
    hooks
        .before_stdin_join()
        .map_err(|_| DarwinProcessFailure::Capture)?;
    let captures = session
        .captures
        .take()
        .ok_or(DarwinProcessFailure::Capture)?;
    let (stdout, stderr) = match join_capture(captures) {
        Ok(output) => output,
        Err(failure) => {
            session.captures = Some(failure.handles);
            return Err(DarwinProcessFailure::Capture);
        }
    };
    if stdout.overflow || stderr.overflow {
        return Err(DarwinProcessFailure::OutputOverflow);
    }
    let descendants = group_exists(session.process.group());
    hooks
        .before_cleanup()
        .map_err(|_| DarwinProcessFailure::Cleanup)?;
    cleanup_process(session.process.pid, session.process.group(), session.reaped)
        .map_err(|_| DarwinProcessFailure::Cleanup)?;
    Ok(DarwinProcessResult {
        termination: if descendants {
            DarwinProcessTermination::DescendantSurvived
        } else {
            status_code(status)
        },
        stdout: stdout.retained,
        stderr_sha256: stderr.digest,
        output_byte_length: stdout.bytes.saturating_add(stderr.bytes),
    })
}

fn status_code(status: i32) -> DarwinProcessTermination {
    if libc::WIFEXITED(status) {
        DarwinProcessTermination::Exited(libc::WEXITSTATUS(status))
    } else if libc::WIFSIGNALED(status) {
        DarwinProcessTermination::Signaled(libc::WTERMSIG(status))
    } else {
        DarwinProcessTermination::Signaled(0)
    }
}
