use super::*;

pub(crate) fn observe_process(
    configured: ConfiguredProcess,
    program: &PinnedExecutable,
    root: &RootAnchor,
    outputs: &OutputConfinement,
    timeout: Duration,
    cancellation: &RoutineCancellation,
) -> Result<ProcessObservation, RoutineError> {
    let ConfiguredProcess {
        running,
        process_group,
        observed,
        overflow,
    } = configured;
    running.observe(|running| {
        running.resume()?;
        let deadline = Instant::now() + timeout;
        let observed_termination =
            observe_termination(running, process_group, &overflow, cancellation, deadline)?;
        let (stdout, stderr) = running.join_io()?;
        program.validate()?;
        root.validate()?;
        outputs.validate()?;
        let termination = normalize_termination(observed_termination, &stdout, &stderr, &overflow);
        Ok(ProcessObservation {
            termination,
            stdout: stdout.retained,
            stderr_sha256: stderr.sha256,
            output_byte_length: observed.load(Ordering::Acquire),
            started: true,
        })
    })
}

fn observe_termination(
    running: &mut RunningProcess,
    process_group: ProcessGroupId,
    overflow: &AtomicBool,
    cancellation: &RoutineCancellation,
    deadline: Instant,
) -> Result<ProcessTermination, RoutineError> {
    loop {
        if cancellation.is_cancelled() {
            return Ok(terminate(running, ProcessTermination::Cancelled));
        }
        if overflow.load(Ordering::Acquire) {
            return Ok(terminate(running, ProcessTermination::OutputLimit));
        }
        if Instant::now() >= deadline {
            return Ok(terminate(running, ProcessTermination::TimedOut));
        }
        maybe_inject_process_failure(ProcessFailurePoint::Wait, "mediator-process-wait-injected")?;
        match running.child_mut()?.try_wait() {
            Ok(Some(status)) => {
                let natural = status_kind(status);
                if process_group_exists(process_group)? {
                    return Ok(terminate_group_after_parent_exit(process_group)
                        .map(|()| ProcessTermination::DescendantSurvived)
                        .unwrap_or(ProcessTermination::CleanupFailed));
                }
                return Ok(natural);
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(2)),
            Err(_) => {
                let cleanup = running.terminate_and_reap();
                return Ok(match cleanup {
                    Ok(()) | Err(_) => ProcessTermination::CleanupFailed,
                });
            }
        }
    }
}

fn terminate(running: &mut RunningProcess, success: ProcessTermination) -> ProcessTermination {
    running
        .terminate_and_reap()
        .map(|()| success)
        .unwrap_or(ProcessTermination::CleanupFailed)
}

fn normalize_termination(
    observed: ProcessTermination,
    stdout: &Drained,
    stderr: &Drained,
    overflow: &AtomicBool,
) -> ProcessTermination {
    match observed {
        ProcessTermination::OutputLimit => ProcessTermination::OutputLimit,
        ProcessTermination::Exited(_) | ProcessTermination::Signaled(_)
            if overflow.load(Ordering::Acquire) =>
        {
            ProcessTermination::OutputLimit
        }
        _ if !stdout.closed || !stderr.closed => ProcessTermination::CleanupFailed,
        value => value,
    }
}
