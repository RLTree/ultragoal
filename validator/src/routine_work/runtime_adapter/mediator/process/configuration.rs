use super::*;

pub(crate) struct ConfiguredProcess {
    pub(crate) running: RunningProcess,
    pub(crate) process_group: ProcessGroupId,
    pub(crate) observed: Arc<AtomicU64>,
    pub(crate) overflow: Arc<AtomicBool>,
}

pub(crate) fn configure_process(
    setup: SpawnSetupGuard,
    root: &RootAnchor,
    outputs: &OutputConfinement,
    framed_input: Vec<u8>,
    output_budget: u64,
) -> Result<ConfiguredProcess, RoutineError> {
    let (setup, (process_group, observed, overflow)) = setup.configure(|setup| {
        root.validate()?;
        outputs.validate()?;
        let process_group = setup.process_group()?;
        inject(
            ProcessFailurePoint::ProcessGroup,
            "mediator-process-group-setup-injected",
        )?;
        configure_output_pipes(setup)?;
        let observed = Arc::new(AtomicU64::new(0));
        let overflow = Arc::new(AtomicBool::new(false));
        start_output_readers(setup, output_budget, &observed, &overflow)?;
        inject(
            ProcessFailurePoint::StdinWriterStart,
            "mediator-stdin-writer-start-injected",
        )?;
        let stdin = setup
            .stdin
            .take()
            .ok_or_else(|| mediator_error("mediator-stdin-unavailable"))?;
        setup.stdin_writer = Some(start_input_writer(stdin, framed_input)?);
        Ok((process_group, observed, overflow))
    })?;
    Ok(ConfiguredProcess {
        running: setup.into_running(process_group),
        process_group,
        observed,
        overflow,
    })
}

fn configure_output_pipes(setup: &SpawnSetupGuard) -> Result<(), RoutineError> {
    inject(
        ProcessFailurePoint::StdoutNonblocking,
        "mediator-stdout-nonblocking-injected",
    )?;
    set_nonblocking(
        setup
            .stdout
            .as_ref()
            .ok_or_else(|| mediator_error("mediator-stdout-unavailable"))?,
    )?;
    inject(
        ProcessFailurePoint::StderrNonblocking,
        "mediator-stderr-nonblocking-injected",
    )?;
    set_nonblocking(
        setup
            .stderr
            .as_ref()
            .ok_or_else(|| mediator_error("mediator-stderr-unavailable"))?,
    )
}

fn start_output_readers(
    setup: &mut SpawnSetupGuard,
    output_budget: u64,
    observed: &Arc<AtomicU64>,
    overflow: &Arc<AtomicBool>,
) -> Result<(), RoutineError> {
    let stdout = take_output_pipe(
        setup,
        ProcessFailurePoint::StdoutReaderStart,
        "mediator-stdout-reader-start-injected",
        true,
    )?;
    let stdout_observed = Arc::clone(observed);
    let stdout_overflow = Arc::clone(overflow);
    let stdout_done = Arc::clone(&setup.readers_done);
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
    let stderr = take_output_pipe(
        setup,
        ProcessFailurePoint::StderrReaderStart,
        "mediator-stderr-reader-start-injected",
        false,
    )?;
    let stderr_observed = Arc::clone(observed);
    let stderr_overflow = Arc::clone(overflow);
    let stderr_done = Arc::clone(&setup.readers_done);
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
    Ok(())
}

fn take_output_pipe(
    setup: &mut SpawnSetupGuard,
    point: ProcessFailurePoint,
    cause: &'static str,
    stdout: bool,
) -> Result<File, RoutineError> {
    inject(point, cause)?;
    let pipe = if stdout {
        &mut setup.stdout
    } else {
        &mut setup.stderr
    };
    pipe.take()
        .ok_or_else(|| mediator_error("mediator-output-pipe-unavailable"))
}

fn inject(point: ProcessFailurePoint, cause: &'static str) -> Result<(), RoutineError> {
    maybe_inject_process_failure(point, cause)
}
