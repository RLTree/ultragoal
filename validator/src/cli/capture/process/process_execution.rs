use super::*;

#[derive(Debug, serde::Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(crate) enum Termination {
    Exited { code: i32 },
    Signaled { signal: i32 },
    TimedOut,
    Interrupted,
    OutputLimit,
    WithheldSecretBearingInvocation,
}

pub(crate) struct ProcessResult {
    pub termination: Termination,
    pub duration_ns: u64,
    pub stdout: CapturedOutput,
    pub stderr: CapturedOutput,
}

type ProcessExecutor = for<'a> fn(
    &'a PinnedProgram,
    &'a PinnedDirectory,
    &'a PreparedEnvironment,
    Duration,
    usize,
    usize,
    Option<&'a Arc<AtomicBool>>,
    &'a SandboxPlan,
) -> Result<ProcessResult, String>;

pub(crate) const EXECUTE: ProcessExecutor = |program,
                                             cwd,
                                             environment,
                                             timeout,
                                             output_limit,
                                             observed_output_limit,
                                             interrupt,
                                             sandbox| {
    execute_request(ProcessExecution {
        program,
        cwd,
        environment,
        timeout,
        output_limit,
        observed_output_limit,
        interrupt,
        sandbox,
    })
};
pub(crate) use EXECUTE as execute;

struct ProcessExecution<'a> {
    program: &'a PinnedProgram,
    cwd: &'a PinnedDirectory,
    environment: &'a PreparedEnvironment,
    timeout: Duration,
    output_limit: usize,
    observed_output_limit: usize,
    interrupt: Option<&'a Arc<AtomicBool>>,
    sandbox: &'a SandboxPlan,
}

fn execute_request(request: ProcessExecution<'_>) -> Result<ProcessResult, String> {
    let ProcessExecution {
        program,
        cwd,
        environment,
        timeout,
        output_limit,
        observed_output_limit,
        interrupt,
        sandbox,
    } = request;
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
        // SAFETY: the closure runs only in the child after fork and before exec;
        // `cwd_fd` is an owned pinned directory descriptor that remains open.
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

pub(crate) fn public_termination(
    termination: Termination,
    sensitivity: InvocationSensitivity,
) -> Termination {
    if sensitivity.is_secret_bearing() {
        Termination::WithheldSecretBearingInvocation
    } else {
        termination
    }
}

#[cfg(unix)]
pub(crate) fn reconcile_termination(
    observed: Termination,
    output_limit_exceeded: bool,
) -> Termination {
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
pub(crate) fn status_kind(status: ExitStatus) -> Termination {
    status.code().map_or_else(
        || Termination::Signaled {
            signal: status.signal().unwrap_or(0),
        },
        |code| Termination::Exited { code },
    )
}

#[cfg(unix)]
pub(crate) fn signal_group(pid: u32, signal: i32) -> Result<(), String> {
    let pid = i32::try_from(pid).map_err(|_| "capture process id overflow".to_owned())?;
    if pid <= 1 {
        return Err("capture process-group cleanup refused unsafe pid".to_owned());
    }
    // SAFETY: `-pid` is a validated negative process-group id and `signal` is
    // supplied only by the fixed termination protocol.
    if unsafe { libc::kill(-pid, signal) } == 0
        || std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
    {
        return Ok(());
    }
    Err("capture process-group signal failed".to_owned())
}
