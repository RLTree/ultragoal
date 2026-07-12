use super::environment::{InvocationSensitivity, PreparedEnvironment};
use super::filesystem::PinnedDirectory;
use super::output::{self, CapturedOutput, OutputBudget};
use super::program::PinnedProgram;
use super::sandbox::SandboxPlan;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::unix::process::{CommandExt, ExitStatusExt};

#[derive(Debug, serde::Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(super) enum Termination {
    Exited { code: i32 },
    Signaled { signal: i32 },
    TimedOut,
    Interrupted,
    OutputLimit,
    WithheldSecretBearingInvocation,
}

pub(super) struct ProcessResult {
    pub termination: Termination,
    pub duration_ns: u64,
    pub stdout: CapturedOutput,
    pub stderr: CapturedOutput,
}

pub(super) fn execute(
    program: &PinnedProgram,
    cwd: &PinnedDirectory,
    environment: &PreparedEnvironment,
    timeout: Duration,
    output_limit: usize,
    observed_output_limit: usize,
    interrupt: Option<&Arc<AtomicBool>>,
    sandbox: &SandboxPlan,
) -> Result<ProcessResult, String> {
    #[cfg(not(unix))]
    {
        let _ = (
            program,
            cwd,
            environment,
            timeout,
            output_limit,
            observed_output_limit,
            interrupt,
            sandbox,
        );
        return Err("capture process execution requires Unix".to_owned());
    }

    #[cfg(unix)]
    {
        // Classification is immutable and complete before the process or either
        // output pipe exists. Readers receive only this classified budget; they
        // cannot independently declassify bytes produced from bound secrets.
        let budget = Arc::new(OutputBudget::for_sensitivity(
            observed_output_limit,
            environment.sensitivity,
        ));
        if interrupt.is_some_and(|flag| flag.load(Ordering::SeqCst)) {
            return Ok(ProcessResult {
                termination: public_termination(Termination::Interrupted, environment.sensitivity),
                duration_ns: 0,
                stdout: budget.empty_output(),
                stderr: budget.empty_output(),
            });
        }
        let mut command = Command::new(sandbox.executable());
        command
            .arg("-D")
            .arg(format!("PROHIBITED_ENV={}", sandbox.prohibited_env()))
            .arg("-p")
            .arg(sandbox.profile())
            .arg(program.path())
            .args(&environment.arguments)
            .env_clear()
            .envs(environment.values.iter().cloned())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let cwd_fd = cwd.raw_fd();
        unsafe {
            command.pre_exec(move || {
                if libc::setpgid(0, 0) != 0 || libc::fchdir(cwd_fd) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let started = Instant::now();
        let mut child = command
            .spawn()
            .map_err(|_| "capture process launch failed".to_owned())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "capture stdout pipe unavailable".to_owned())?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| "capture stderr pipe unavailable".to_owned())?;
        let stdout_budget = Arc::clone(&budget);
        let stderr_budget = Arc::clone(&budget);
        let stdout_reader =
            std::thread::spawn(move || output::observe(stdout, output_limit, &stdout_budget));
        let stderr_reader =
            std::thread::spawn(move || output::observe(stderr, output_limit, &stderr_budget));
        let deadline = started + timeout;
        let observed_termination = loop {
            if interrupt.is_some_and(|flag| flag.load(Ordering::SeqCst)) {
                terminate_and_reap(&mut child)?;
                break Termination::Interrupted;
            }
            if budget.exceeded() {
                terminate_and_reap(&mut child)?;
                break Termination::OutputLimit;
            }
            if Instant::now() >= deadline {
                terminate_and_reap(&mut child)?;
                break Termination::TimedOut;
            }
            match child.try_wait() {
                Ok(Some(status)) => break status_kind(status),
                Ok(None) => std::thread::sleep(Duration::from_millis(2)),
                Err(_) => {
                    terminate_and_reap(&mut child)?;
                    return Err("capture process status failed".to_owned());
                }
            }
        };
        let stdout_result = stdout_reader.join();
        let stderr_result = stderr_reader.join();
        let stdout = stdout_result.map_err(|_| "capture stdout reader failed".to_owned())??;
        let stderr = stderr_result.map_err(|_| "capture stderr reader failed".to_owned())??;
        let outputs = budget.finalize_streams(stdout, stderr);
        let termination =
            reconcile_termination(observed_termination, outputs.output_limit_exceeded);
        let withheld = environment.sensitivity.is_secret_bearing();
        Ok(ProcessResult {
            termination: public_termination(termination, environment.sensitivity),
            duration_ns: if withheld {
                0
            } else {
                started.elapsed().as_nanos().min(u64::MAX as u128) as u64
            },
            stdout: outputs.first,
            stderr: outputs.second,
        })
    }
}

fn public_termination(termination: Termination, sensitivity: InvocationSensitivity) -> Termination {
    if sensitivity.is_secret_bearing() {
        Termination::WithheldSecretBearingInvocation
    } else {
        termination
    }
}

#[cfg(unix)]
fn reconcile_termination(observed: Termination, output_limit_exceeded: bool) -> Termination {
    // A control cause already observed by the parent remains authoritative.
    // Natural exit or signal can race with the readers (including SIGPIPE after
    // a bounded reader closes), so a subsequently confirmed overflow wins.
    match observed {
        Termination::Exited { .. } | Termination::Signaled { .. } if output_limit_exceeded => {
            Termination::OutputLimit
        }
        control_cause => control_cause,
    }
}

#[cfg(unix)]
fn status_kind(status: ExitStatus) -> Termination {
    status.code().map_or_else(
        || Termination::Signaled {
            signal: status.signal().unwrap_or(0),
        },
        |code| Termination::Exited { code },
    )
}

#[cfg(unix)]
fn signal_group(pid: u32, signal: i32) -> Result<(), String> {
    let pid = i32::try_from(pid).map_err(|_| "capture process id overflow".to_owned())?;
    if pid <= 1 {
        return Err("capture process-group cleanup refused unsafe pid".to_owned());
    }
    if unsafe { libc::kill(-pid, signal) } == 0
        || std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
    {
        return Ok(());
    }
    Err("capture process-group signal failed".to_owned())
}

#[cfg(unix)]
fn terminate_and_reap(child: &mut std::process::Child) -> Result<(), String> {
    if signal_group(child.id(), libc::SIGTERM).is_err() {
        if child
            .try_wait()
            .map_err(|_| "capture process cleanup status failed".to_owned())?
            .is_some()
        {
            return Ok(());
        }
        child
            .kill()
            .map_err(|_| "capture process direct cleanup failed".to_owned())?;
        return child
            .wait()
            .map(|_| ())
            .map_err(|_| "capture process could not be reaped".to_owned());
    }
    let deadline = Instant::now() + Duration::from_millis(50);
    while Instant::now() < deadline {
        match child.try_wait() {
            Ok(Some(_)) => return Ok(()),
            Ok(None) => std::thread::sleep(Duration::from_millis(2)),
            Err(_) => return Err("capture process cleanup status failed".to_owned()),
        }
    }
    if signal_group(child.id(), libc::SIGKILL).is_err() {
        let _ = child.kill();
    }
    child
        .wait()
        .map(|_| ())
        .map_err(|_| "capture process could not be reaped".to_owned())
}

#[cfg(test)]
mod tests {
    use super::{Termination, reconcile_termination, signal_group, terminate_and_reap};
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    use std::os::unix::process::CommandExt;

    #[test]
    fn cleanup_terminates_and_reaps_a_process_group() {
        let mut command = Command::new("/bin/sh");
        command
            .args(["-c", "while :; do :; done"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        unsafe {
            command.pre_exec(|| {
                if libc::setpgid(0, 0) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut child = command.spawn().unwrap();
        let started = Instant::now();
        terminate_and_reap(&mut child).unwrap();
        assert!(started.elapsed() < Duration::from_secs(1));
        assert!(child.try_wait().unwrap().is_some());
        assert!(signal_group(child.id(), libc::SIGKILL).is_ok());
    }

    #[test]
    fn output_overflow_reconciliation_preserves_observed_control_causes() {
        assert!(matches!(
            reconcile_termination(Termination::Exited { code: 0 }, true),
            Termination::OutputLimit
        ));
        assert!(matches!(
            reconcile_termination(
                Termination::Signaled {
                    signal: libc::SIGPIPE
                },
                true
            ),
            Termination::OutputLimit
        ));
        assert!(matches!(
            reconcile_termination(Termination::TimedOut, true),
            Termination::TimedOut
        ));
        assert!(matches!(
            reconcile_termination(Termination::Interrupted, true),
            Termination::Interrupted
        ));
        assert!(matches!(
            reconcile_termination(Termination::Exited { code: 0 }, false),
            Termination::Exited { code: 0 }
        ));
    }
}
