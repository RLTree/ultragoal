use super::*;

pub(crate) enum PreparedProcess {
    Cancelled(ProcessObservation),
    Suspended(SuspendedProcess),
}

pub(crate) struct SuspendedProcess {
    setup: SpawnSetupGuard,
    framed_input: Vec<u8>,
    output_budget: u64,
}

pub(crate) fn prepare(
    program: &PinnedExecutable,
    root: &RootAnchor,
    outputs: &OutputConfinement,
    reads: &ReadConfinement,
    argv: &[String],
    environment: &BTreeMap<String, String>,
    framed_input: Vec<u8>,
    output_budget: u64,
    cancellation: &RoutineCancellation,
) -> Result<PreparedProcess, RoutineError> {
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
            output_budget,
            cancellation,
        );
        return Err(mediator_error("mediator-confinement-substrate-unavailable"));
    }
    #[cfg(target_os = "macos")]
    {
        validate_child_mode(environment)?;
        if cancellation.is_cancelled() {
            return Ok(PreparedProcess::Cancelled(cancelled_before_spawn()));
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
            return Ok(PreparedProcess::Cancelled(cancelled_before_spawn()));
        }
        let setup = spawn_exact_program(program, root, argv, environment)?;
        Ok(PreparedProcess::Suspended(SuspendedProcess {
            setup,
            framed_input,
            output_budget,
        }))
    }
}

impl SuspendedProcess {
    pub(crate) fn identity(&self) -> Result<StartedProcessIdentity, RoutineError> {
        Ok(StartedProcessIdentity::new(
            self.setup.child()?,
            self.setup.process_group()?,
        ))
    }

    pub(crate) fn observe(
        self,
        program: &PinnedExecutable,
        root: &RootAnchor,
        outputs: &OutputConfinement,
        timeout: Duration,
        cancellation: &RoutineCancellation,
    ) -> Result<ProcessObservation, RoutineError> {
        let configured = configure_process(
            self.setup,
            root,
            outputs,
            self.framed_input,
            self.output_budget,
        )?;
        observe_process(configured, program, root, outputs, timeout, cancellation)
    }

    pub(crate) fn fail(self, primary: RoutineError) -> RoutineError {
        match self.setup.configure::<()>(|_| Err(primary)) {
            Err(error) => error,
            Ok(_) => mediator_error("mediator-suspended-cleanup-outcome-invalid"),
        }
    }

    pub(crate) fn resume_after_cleanup(self, payload: Box<dyn std::any::Any + Send>) -> ! {
        match self
            .setup
            .configure::<()>(|_| std::panic::resume_unwind(payload))
        {
            Ok(_) | Err(_) => {
                std::panic::resume_unwind(Box::new("mediator-suspended-cleanup-outcome-invalid"))
            }
        }
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
    }
}
