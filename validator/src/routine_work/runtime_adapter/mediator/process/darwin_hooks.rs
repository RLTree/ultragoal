use super::*;

pub(super) struct RoutineDarwinHooks;

impl crate::process_custody::DarwinProcessHooks for RoutineDarwinHooks {
    fn before_resume(&mut self) -> std::io::Result<()> {
        hook(
            ProcessFailurePoint::Resume,
            "mediator-process-resume-injected",
        )
    }

    fn before_wait(&mut self) -> std::io::Result<()> {
        hook(ProcessFailurePoint::Wait, "mediator-process-wait-injected")
    }

    fn before_stdout_join(&mut self) -> std::io::Result<()> {
        hook(
            ProcessFailurePoint::StdoutJoin,
            "mediator-stdout-reader-join-injected",
        )
    }

    fn before_stderr_join(&mut self) -> std::io::Result<()> {
        hook(
            ProcessFailurePoint::StderrJoin,
            "mediator-stderr-reader-join-injected",
        )
    }

    fn before_stdin_join(&mut self) -> std::io::Result<()> {
        hook(
            ProcessFailurePoint::StdinJoin,
            "mediator-stdin-writer-join-injected",
        )
    }

    fn before_cleanup(&mut self) -> std::io::Result<()> {
        hook(
            ProcessFailurePoint::Cleanup,
            "mediator-process-cleanup-injected",
        )
    }
}

fn hook(point: ProcessFailurePoint, cause: &'static str) -> std::io::Result<()> {
    maybe_inject_process_failure(point, cause).map_err(|_| std::io::Error::other(cause))
}

pub(super) fn validate_child_mode(
    environment: &std::collections::BTreeMap<String, String>,
) -> Result<(), RoutineError> {
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

pub(super) fn inject(point: ProcessFailurePoint, cause: &'static str) -> Result<(), RoutineError> {
    maybe_inject_process_failure(point, cause)
}
