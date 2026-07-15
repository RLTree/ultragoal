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
        validate_child_mode(environment)?;
        if cancellation.is_cancelled() {
            return Ok(cancelled_before_spawn());
        }
        program.validate()?;
        let profile = sandbox_profile(
            root.path(),
            &reads.absolute_sources(),
            &outputs.absolute_scopes(),
        )?;
        let framed_input = crate::routine_work::frame_sandboxed_input(&profile, framed_input)
            .map_err(mediator_error)?;
        root.validate()?;
        outputs.validate()?;
        if cancellation.is_cancelled() {
            return Ok(cancelled_before_spawn());
        }
        let setup = spawn_exact_program(program, root, argv, environment)?;
        let configured = configure_process(
            setup,
            root,
            outputs,
            framed_input,
            output_budget,
            on_started,
        )?;
        observe_process(configured, program, root, outputs, timeout, cancellation)
    }
}

#[cfg(target_os = "macos")]
fn validate_child_mode(environment: &BTreeMap<String, String>) -> Result<(), RoutineError> {
    if environment
        .get(crate::routine_work::CHILD_MODE_ENV)
        .map(String::as_str)
        == Some(crate::routine_work::CHILD_MODE_VALUE)
    {
        Ok(())
    } else {
        Err(mediator_error("mediator-child-mode-binding-invalid"))
    }
}

#[cfg(target_os = "macos")]
fn cancelled_before_spawn() -> ProcessObservation {
    ProcessObservation {
        termination: ProcessTermination::Cancelled,
        stdout: Vec::new(),
        stderr_sha256: digest_bytes(&[]),
        output_byte_length: 0,
        started: false,
    }
}
