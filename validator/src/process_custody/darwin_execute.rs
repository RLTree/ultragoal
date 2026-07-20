use super::darwin::spawn_suspended_descriptor;
use super::darwin_cleanup::cleanup_process;
use super::darwin_process::{
    DarwinNoopHooks, DarwinProcessFailure, DarwinProcessPolicy, DarwinProcessTermination,
    execute_process,
};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

#[derive(Clone, Copy)]
pub(crate) struct DarwinExecutionPolicy {
    pub(crate) timeout: Duration,
    pub(crate) output_limit: usize,
}

pub(crate) struct DarwinCommandCapture {
    pub(crate) exit_code: i32,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DarwinExecutionFailure {
    Spawn,
    LoadedVnode,
    Resume,
    Cancelled,
    Timeout,
    OutputOverflow,
    Process,
    Cleanup,
}

pub(crate) fn execute_suspended_descriptor(
    program: &Path,
    device: u64,
    inode: u64,
    cwd: std::os::fd::RawFd,
    argv: &[String],
    environment: &BTreeMap<String, String>,
    policy: DarwinExecutionPolicy,
    cancelled: impl Fn() -> bool,
) -> Result<DarwinCommandCapture, DarwinExecutionFailure> {
    let process = spawn_suspended_descriptor(program, cwd, argv, environment)
        .map_err(|_| DarwinExecutionFailure::Spawn)?;
    if process
        .validate_loaded_vnode(program, device, inode)
        .is_err()
    {
        let cleanup = cleanup_process(process.pid, process.group(), false);
        return Err(if cleanup.is_err() {
            DarwinExecutionFailure::Cleanup
        } else {
            DarwinExecutionFailure::LoadedVnode
        });
    }
    let mut hooks = DarwinNoopHooks;
    let result = execute_process(
        process,
        &[],
        DarwinProcessPolicy {
            timeout: policy.timeout,
            stdout_limit: policy.output_limit,
            stderr_limit: policy.output_limit,
        },
        cancelled,
        &mut hooks,
    )
    .map_err(map_failure)?;
    if result.termination == DarwinProcessTermination::DescendantSurvived {
        return Err(DarwinExecutionFailure::Cleanup);
    }
    let exit_code = match result.termination {
        DarwinProcessTermination::Exited(code) => code,
        DarwinProcessTermination::Signaled(signal) => 128 + signal,
        DarwinProcessTermination::DescendantSurvived => -1,
    };
    if exit_code != 0 {
        return Err(DarwinExecutionFailure::Process);
    }
    Ok(DarwinCommandCapture {
        exit_code,
        stdout: result.stdout,
        stderr: result.stderr,
    })
}

fn map_failure(failure: DarwinProcessFailure) -> DarwinExecutionFailure {
    match failure {
        DarwinProcessFailure::Resume => DarwinExecutionFailure::Resume,
        DarwinProcessFailure::Cancelled => DarwinExecutionFailure::Cancelled,
        DarwinProcessFailure::Timeout => DarwinExecutionFailure::Timeout,
        DarwinProcessFailure::OutputOverflow => DarwinExecutionFailure::OutputOverflow,
        DarwinProcessFailure::Cleanup => DarwinExecutionFailure::Cleanup,
        DarwinProcessFailure::Wait | DarwinProcessFailure::Capture => {
            DarwinExecutionFailure::Process
        }
    }
}
