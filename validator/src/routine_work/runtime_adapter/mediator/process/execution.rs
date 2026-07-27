use super::super::AuthorizedProcessPreparation;
#[cfg(target_os = "macos")]
use super::darwin_hooks::{RoutineDarwinHooks, inject, validate_child_mode};
use super::*;
use std::time::Duration;

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
    preparation: AuthorizedProcessPreparation<'_>,
) -> Result<PreparedProcess, RoutineError> {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (
            preparation.program,
            preparation.root,
            preparation.outputs,
            preparation.reads,
            preparation.argv,
            preparation.environment,
            preparation.framed_input,
            preparation.output_budget,
            preparation.cancellation,
        );
        return Err(mediator_error("mediator-confinement-substrate-unavailable"));
    }
    #[cfg(target_os = "macos")]
    {
        validate_child_mode(preparation.environment)?;
        if preparation.cancellation.is_cancelled() {
            return Ok(PreparedProcess::Cancelled(cancelled_before_spawn()));
        }
        preparation.program.validate()?;
        let profile = sandbox_profile(
            preparation.root.path(),
            &preparation.reads.absolute_sources(),
            &preparation.outputs.absolute_scopes(),
        )?;
        let framed_input =
            crate::routine_work::frame_sandboxed_input(&profile, preparation.framed_input)
                .map_err(mediator_error)?;
        preparation.root.validate()?;
        preparation.outputs.validate()?;
        if preparation.cancellation.is_cancelled() {
            return Ok(PreparedProcess::Cancelled(cancelled_before_spawn()));
        }
        let setup = spawn_exact_program(
            preparation.program,
            preparation.root,
            preparation.argv,
            preparation.environment,
        )?;
        Ok(PreparedProcess::Suspended(SuspendedProcess {
            setup,
            framed_input,
            output_budget: preparation.output_budget,
        }))
    }
}

impl SuspendedProcess {
    pub(crate) fn identity(&self) -> Result<StartedProcessIdentity, RoutineError> {
        Ok(StartedProcessIdentity::new(
            self.setup.process_id()?,
            self.setup.process_group()?,
        ))
    }

    pub(crate) fn observe(
        mut self,
        program: &PinnedExecutable,
        root: &RootAnchor,
        outputs: &OutputConfinement,
        timeout: Duration,
        cancellation: &RoutineCancellation,
    ) -> Result<ProcessObservation, RoutineError> {
        let (setup, ()) = self.setup.configure(|_| {
            for (point, cause) in [
                (
                    ProcessFailurePoint::ProcessGroup,
                    "mediator-process-group-setup-injected",
                ),
                (
                    ProcessFailurePoint::StdoutNonblocking,
                    "mediator-stdout-nonblocking-injected",
                ),
                (
                    ProcessFailurePoint::StderrNonblocking,
                    "mediator-stderr-nonblocking-injected",
                ),
                (
                    ProcessFailurePoint::StdoutReaderStart,
                    "mediator-stdout-reader-start-injected",
                ),
                (
                    ProcessFailurePoint::StderrReaderStart,
                    "mediator-stderr-reader-start-injected",
                ),
                (
                    ProcessFailurePoint::StdinWriterStart,
                    "mediator-stdin-writer-start-injected",
                ),
            ] {
                inject(point, cause)?;
            }
            Ok(())
        })?;
        self.setup = setup;
        let (setup, ()) = self.setup.configure(|_| {
            root.validate()?;
            outputs.validate()?;
            Ok(())
        })?;
        self.setup = setup;
        let process = self.setup.take_process()?;
        let mut hooks = RoutineDarwinHooks;
        let result = crate::process_custody::execute_process(
            process,
            &self.framed_input,
            crate::process_custody::DarwinProcessPolicy {
                timeout,
                stdout_limit: self.output_budget as usize,
                stderr_limit: self.output_budget as usize,
            },
            || cancellation.is_cancelled(),
            &mut hooks,
        );
        match result {
            Ok(result) => {
                program.validate()?;
                root.validate()?;
                outputs.validate()?;
                let termination = match result.termination {
                    crate::process_custody::DarwinProcessTermination::Exited(code) => {
                        ProcessTermination::Exited(code)
                    }
                    crate::process_custody::DarwinProcessTermination::Signaled(signal) => {
                        ProcessTermination::Signaled(signal)
                    }
                    crate::process_custody::DarwinProcessTermination::DescendantSurvived => {
                        ProcessTermination::DescendantSurvived
                    }
                };
                Ok(ProcessObservation {
                    termination,
                    stdout: result.stdout,
                    stderr_sha256: result.stderr_sha256,
                    output_byte_length: result.output_byte_length,
                })
            }
            Err(crate::process_custody::DarwinProcessFailure::Cancelled) => {
                Ok(observation(ProcessTermination::Cancelled))
            }
            Err(crate::process_custody::DarwinProcessFailure::Timeout) => {
                Ok(observation(ProcessTermination::TimedOut))
            }
            Err(crate::process_custody::DarwinProcessFailure::OutputOverflow) => {
                Ok(observation(ProcessTermination::OutputLimit))
            }
            Err(crate::process_custody::DarwinProcessFailure::Resume) => {
                Err(mediator_error("mediator-process-resume-failed"))
            }
            Err(crate::process_custody::DarwinProcessFailure::Wait) => {
                Err(mediator_error("mediator-process-wait-failed"))
            }
            Err(crate::process_custody::DarwinProcessFailure::Capture) => {
                Err(mediator_error("mediator-output-capture-failed"))
            }
            Err(crate::process_custody::DarwinProcessFailure::Cleanup) => {
                Ok(observation(ProcessTermination::CleanupFailed))
            }
        }
    }

    pub(crate) fn fail(mut self, primary: RoutineError) -> RoutineError {
        match self.setup.cleanup() {
            Ok(()) => primary,
            Err(error) => error,
        }
    }

    pub(crate) fn resume_after_cleanup(mut self, payload: Box<dyn std::any::Any + Send>) -> ! {
        let _ = self.setup.cleanup();
        std::panic::resume_unwind(payload)
    }
}

#[cfg(target_os = "macos")]
fn cancelled_before_spawn() -> ProcessObservation {
    observation(ProcessTermination::Cancelled)
}

#[cfg(target_os = "macos")]
fn observation(termination: ProcessTermination) -> ProcessObservation {
    ProcessObservation {
        termination,
        stdout: Vec::new(),
        stderr_sha256: digest_bytes(&[]),
        output_byte_length: 0,
    }
}

#[cfg(not(target_os = "macos"))]
fn observation(termination: ProcessTermination) -> ProcessObservation {
    ProcessObservation {
        termination,
        stdout: Vec::new(),
        stderr_sha256: digest_bytes(&[]),
        output_byte_length: 0,
    }
}
