use super::*;
pub(crate) fn execute<F>(
    program: &PinnedExecutable,
    root: &RootAnchor,
    outputs: &OutputConfinement,
    reads: &ReadConfinement,
    argv: &[String],
    environment: &BTreeMap<String, String>,
    framed_input: Vec<u8>,
    timeout: Duration,
    output_budget: u64,
    cancellation: &RoutineCancellation,
    on_started: F,
) -> Result<ProcessObservation, RoutineError>
where
    F: FnOnce() -> Result<(), RoutineError>,
{
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (
            program,
            root,
            outputs,
            reads,
            argv,
            environment,
            framed_input,
            timeout,
            output_budget,
            cancellation,
            on_started,
        );
        return Err(mediator_error("mediator-confinement-substrate-unavailable"));
    }
    #[cfg(target_os = "macos")]
    {
        if environment
            .get(crate::routine_work::CHILD_MODE_ENV)
            .map(String::as_str)
            != Some(crate::routine_work::CHILD_MODE_VALUE)
        {
            return Err(mediator_error("mediator-child-mode-binding-invalid"));
        }
        if cancellation.is_cancelled() {
            return Ok(ProcessObservation {
                termination: ProcessTermination::Cancelled,
                stdout: Vec::new(),
                stderr_sha256: digest_bytes(&[]),
                output_byte_length: 0,
                started: false,
            });
        }
        let sandbox = PinnedExecutable::open_unbound(Path::new("/usr/bin/sandbox-exec"))?;
        program.validate()?;
        sandbox.validate()?;
        sandbox.validate_named_path()?;
        let profile = sandbox_profile(
            program.path(),
            root.path(),
            &reads.absolute_sources(),
            &outputs.absolute_scopes(),
        )?;
        root.validate()?;
        outputs.validate()?;
        let mut command = Command::new(sandbox.path());
        command
            .arg("-p")
            .arg(profile)
            .arg(program.path())
            .args(argv.iter().skip(1))
            .current_dir(root.path())
            .env_clear()
            .envs(environment)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let cwd_fd = root.raw_fd();
        unsafe {
            command.pre_exec(move || {
                if libc::setpgid(0, 0) != 0 || libc::fchdir(cwd_fd) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        if cancellation.is_cancelled() {
            return Ok(ProcessObservation {
                termination: ProcessTermination::Cancelled,
                stdout: Vec::new(),
                stderr_sha256: digest_bytes(&[]),
                output_byte_length: 0,
                started: false,
            });
        }
        run_test_process_pre_spawn_hook();
        let started_at = Instant::now();
        let child = command
            .spawn()
            .map_err(|_| mediator_error("mediator-process-launch-failed"))?;
        run_test_process_post_spawn_hook();
        let mut setup = SpawnSetupGuard::new(child);
        on_started()?;
        #[cfg(test)]
        TEST_SPAWN_COUNT.fetch_add(1, Ordering::SeqCst);
        let process_group = setup.process_group()?;
        maybe_inject_setup_failure(
            SetupFailurePoint::ProcessGroup,
            "mediator-process-group-setup-injected",
        )?;
        setup.take_pipes()?;
        maybe_inject_setup_failure(
            SetupFailurePoint::StdoutNonblocking,
            "mediator-stdout-nonblocking-injected",
        )?;
        set_nonblocking(
            setup
                .stdout
                .as_ref()
                .expect("stdout retained by setup guard"),
        )?;
        maybe_inject_setup_failure(
            SetupFailurePoint::StderrNonblocking,
            "mediator-stderr-nonblocking-injected",
        )?;
        set_nonblocking(
            setup
                .stderr
                .as_ref()
                .expect("stderr retained by setup guard"),
        )?;
        let observed = Arc::new(AtomicU64::new(0));
        let overflow = Arc::new(AtomicBool::new(false));
        let stdout_observed = Arc::clone(&observed);
        let stdout_overflow = Arc::clone(&overflow);
        let stdout_done = Arc::clone(&setup.readers_done);
        maybe_inject_setup_failure(
            SetupFailurePoint::StdoutReaderStart,
            "mediator-stdout-reader-start-injected",
        )?;
        let stdout = setup.stdout.take().expect("validated stdout pipe");
        setup.stdout_reader = Some(
            std::thread::Builder::new()
                .name("routine-mediator-stdout".to_owned())
                .spawn(move || {
                    drain(
                        stdout,
                        output_budget,
                        &stdout_observed,
                        &stdout_overflow,
                        &stdout_done,
                        true,
                    )
                })
                .map_err(|_| mediator_error("mediator-stdout-reader-start-failed"))?,
        );
        let stderr_observed = Arc::clone(&observed);
        let stderr_overflow = Arc::clone(&overflow);
        let stderr_done = Arc::clone(&setup.readers_done);
        maybe_inject_setup_failure(
            SetupFailurePoint::StderrReaderStart,
            "mediator-stderr-reader-start-injected",
        )?;
        let stderr = setup.stderr.take().expect("validated stderr pipe");
        setup.stderr_reader = Some(
            std::thread::Builder::new()
                .name("routine-mediator-stderr".to_owned())
                .spawn(move || {
                    drain(
                        stderr,
                        output_budget,
                        &stderr_observed,
                        &stderr_overflow,
                        &stderr_done,
                        false,
                    )
                })
                .map_err(|_| mediator_error("mediator-stderr-reader-start-failed"))?,
        );
        let stdin = setup
            .stdin
            .take()
            .ok_or_else(|| mediator_error("mediator-stdin-unavailable"))?;
        setup.stdin_writer = Some(start_input_writer(stdin, framed_input)?);
        let mut running = setup.into_running();
        let deadline = started_at + timeout;
        let observed_termination = loop {
            if cancellation.is_cancelled() {
                break running
                    .terminate_and_reap()
                    .map(|_| ProcessTermination::Cancelled)
                    .unwrap_or(ProcessTermination::CleanupFailed);
            }
            if overflow.load(Ordering::Acquire) {
                break running
                    .terminate_and_reap()
                    .map(|_| ProcessTermination::OutputLimit)
                    .unwrap_or(ProcessTermination::CleanupFailed);
            }
            if Instant::now() >= deadline {
                break running
                    .terminate_and_reap()
                    .map(|_| ProcessTermination::TimedOut)
                    .unwrap_or(ProcessTermination::CleanupFailed);
            }
            match running.child_mut().try_wait() {
                Ok(Some(status)) => {
                    let natural = status_kind(status);
                    if process_group_exists(process_group)? {
                        break terminate_group_after_parent_exit(process_group)
                            .map(|_| ProcessTermination::DescendantSurvived)
                            .unwrap_or(ProcessTermination::CleanupFailed);
                    }
                    break natural;
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(2)),
                Err(_) => {
                    let cleanup = running.terminate_and_reap();
                    break if cleanup.is_ok() {
                        ProcessTermination::CleanupFailed
                    } else {
                        ProcessTermination::CleanupFailed
                    };
                }
            }
        };
        let (stdout, stderr) = running.join_io()?;
        program.validate()?;
        sandbox.validate()?;
        root.validate()?;
        outputs.validate()?;
        let termination = match observed_termination {
            ProcessTermination::OutputLimit => ProcessTermination::OutputLimit,
            ProcessTermination::Exited(_) | ProcessTermination::Signaled(_)
                if overflow.load(Ordering::Acquire) =>
            {
                ProcessTermination::OutputLimit
            }
            _ if !stdout.closed || !stderr.closed => ProcessTermination::CleanupFailed,
            value => value,
        };
        running.disarm();
        Ok(ProcessObservation {
            termination,
            stdout: stdout.retained,
            stderr_sha256: stderr.sha256,
            output_byte_length: observed.load(Ordering::Acquire),
            started: true,
        })
    }
}
