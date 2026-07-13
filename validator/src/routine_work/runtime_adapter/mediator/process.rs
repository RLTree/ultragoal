use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStderr, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
#[cfg(test)]
use std::sync::{Mutex, OnceLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::fd::AsRawFd;
#[cfg(unix)]
use std::os::unix::process::{CommandExt, ExitStatusExt};

use crate::routine_work::{RoutineError, RoutineErrorId};

use super::filesystem::{OutputConfinement, PinnedExecutable, ReadConfinement, RootAnchor};
use super::model::RoutineCancellation;

#[cfg(test)]
static TEST_SPAWN_COUNT: AtomicU64 = AtomicU64::new(0);

pub(super) struct ProcessObservation {
    pub(super) termination: ProcessTermination,
    pub(super) stdout: Vec<u8>,
    pub(super) stderr_sha256: String,
    pub(super) output_byte_length: u64,
    pub(super) started: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ProcessTermination {
    Exited(i32),
    Signaled(i32),
    TimedOut,
    Cancelled,
    OutputLimit,
    DescendantSurvived,
    CleanupFailed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SetupFailurePoint {
    ProcessGroup,
    StdoutNonblocking,
    StderrNonblocking,
    StdoutReaderStart,
    StderrReaderStart,
}

type ReaderHandle = JoinHandle<Result<Drained, RoutineError>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ProcessGroupId(i32);

impl ProcessGroupId {
    fn from_child_id(child_id: u32) -> Result<Self, RoutineError> {
        let value = i32::try_from(child_id)
            .map_err(|_| mediator_error("mediator-process-group-invalid"))?;
        Self::new(value)
    }

    fn new(value: i32) -> Result<Self, RoutineError> {
        if value <= 0 || value.checked_neg().is_none() {
            return Err(mediator_error("mediator-process-group-invalid"));
        }
        Ok(Self(value))
    }

    fn signal_target(self) -> Result<i32, RoutineError> {
        self.0
            .checked_neg()
            .filter(|target| *target < 0)
            .ok_or_else(|| mediator_error("mediator-process-group-invalid"))
    }
}

/// Owns every spawned-process resource until all fallible setup completes.
///
/// It is installed immediately after `spawn()`. Any return or unwind before
/// `into_running` terminates and reaps the process group, closes unhanded pipe
/// descriptors, signals started readers to stop, and joins them.
struct SpawnSetupGuard {
    child: Option<Child>,
    process_group: Option<ProcessGroupId>,
    stdout: Option<ChildStdout>,
    stderr: Option<ChildStderr>,
    readers_done: Arc<AtomicBool>,
    stdout_reader: Option<ReaderHandle>,
    stderr_reader: Option<ReaderHandle>,
    cleanup_required: bool,
}

impl SpawnSetupGuard {
    fn new(child: Child) -> Self {
        let process_group = ProcessGroupId::from_child_id(child.id()).ok();
        Self {
            child: Some(child),
            process_group,
            stdout: None,
            stderr: None,
            readers_done: Arc::new(AtomicBool::new(false)),
            stdout_reader: None,
            stderr_reader: None,
            cleanup_required: true,
        }
    }

    fn process_group(&self) -> Result<ProcessGroupId, RoutineError> {
        self.process_group
            .ok_or_else(|| mediator_error("mediator-process-group-invalid"))
    }

    fn take_pipes(&mut self) -> Result<(), RoutineError> {
        let child = self
            .child
            .as_mut()
            .expect("spawn guard always owns its child before handoff");
        self.stdout = Some(
            child
                .stdout
                .take()
                .ok_or_else(|| mediator_error("mediator-stdout-unavailable"))?,
        );
        self.stderr = Some(
            child
                .stderr
                .take()
                .ok_or_else(|| mediator_error("mediator-stderr-unavailable"))?,
        );
        Ok(())
    }

    fn into_running(mut self) -> RunningProcess {
        let running = RunningProcess {
            child: self.child.take(),
            process_group: self
                .process_group
                .take()
                .expect("validated process group before setup handoff"),
            readers_done: Arc::clone(&self.readers_done),
            stdout_reader: self.stdout_reader.take(),
            stderr_reader: self.stderr_reader.take(),
            cleanup_required: true,
        };
        self.cleanup_required = false;
        running
    }
}

impl Drop for SpawnSetupGuard {
    fn drop(&mut self) {
        if !self.cleanup_required {
            return;
        }
        self.readers_done.store(true, Ordering::Release);
        if let Some(child) = self.child.as_mut() {
            let _ = cleanup_spawned_child(child, self.process_group);
        }
        self.stdout.take();
        self.stderr.take();
        join_reader_best_effort(&mut self.stdout_reader);
        join_reader_best_effort(&mut self.stderr_reader);
    }
}

/// Owns child, group, and reader threads for all post-setup fallible work.
/// Dropping it before `disarm` repeats fail-closed cleanup and joins readers.
struct RunningProcess {
    child: Option<Child>,
    process_group: ProcessGroupId,
    readers_done: Arc<AtomicBool>,
    stdout_reader: Option<ReaderHandle>,
    stderr_reader: Option<ReaderHandle>,
    cleanup_required: bool,
}

impl RunningProcess {
    fn child_mut(&mut self) -> &mut Child {
        self.child
            .as_mut()
            .expect("running process retains child ownership")
    }

    fn terminate_and_reap(&mut self) -> Result<(), RoutineError> {
        let process_group = self.process_group;
        terminate_and_reap(self.child_mut(), process_group)
    }

    fn join_readers(&mut self) -> Result<(Drained, Drained), RoutineError> {
        self.readers_done.store(true, Ordering::Release);
        let stdout = self
            .stdout_reader
            .take()
            .expect("stdout reader installed before setup handoff")
            .join()
            .map_err(|_| mediator_error("mediator-stdout-reader-failed"))??;
        let stderr = self
            .stderr_reader
            .take()
            .expect("stderr reader installed before setup handoff")
            .join()
            .map_err(|_| mediator_error("mediator-stderr-reader-failed"))??;
        Ok((stdout, stderr))
    }

    fn disarm(&mut self) {
        debug_assert!(self.stdout_reader.is_none() && self.stderr_reader.is_none());
        self.cleanup_required = false;
    }
}

impl Drop for RunningProcess {
    fn drop(&mut self) {
        if !self.cleanup_required {
            return;
        }
        self.readers_done.store(true, Ordering::Release);
        if let Some(child) = self.child.as_mut() {
            let _ = cleanup_spawned_child(child, Some(self.process_group));
        }
        join_reader_best_effort(&mut self.stdout_reader);
        join_reader_best_effort(&mut self.stderr_reader);
    }
}

fn join_reader_best_effort(reader: &mut Option<ReaderHandle>) {
    if let Some(reader) = reader.take() {
        let _ = reader.join();
    }
}

pub(super) fn execute<F>(
    program: &PinnedExecutable,
    root: &RootAnchor,
    outputs: &OutputConfinement,
    reads: &ReadConfinement,
    argv: &[String],
    environment: &BTreeMap<String, String>,
    timeout: Duration,
    output_budget: u64,
    cancellation: &RoutineCancellation,
    on_started: F,
) -> Result<ProcessObservation, RoutineError>
where
    F: FnOnce(),
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
            timeout,
            output_budget,
            cancellation,
            on_started,
        );
        return Err(mediator_error("mediator-confinement-substrate-unavailable"));
    }
    #[cfg(target_os = "macos")]
    {
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
        let profile = sandbox_profile(
            program.path(),
            root.path(),
            &reads.absolute_sources(),
            &outputs.absolute_scopes(),
        )?;
        program.validate()?;
        sandbox.validate()?;
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
            .stdin(Stdio::null())
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
        let started_at = Instant::now();
        let child = command
            .spawn()
            .map_err(|_| mediator_error("mediator-process-launch-failed"))?;
        let mut setup = SpawnSetupGuard::new(child);
        on_started();
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
        let (stdout, stderr) = running.join_readers()?;
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

struct Drained {
    retained: Vec<u8>,
    sha256: String,
    closed: bool,
}

fn drain(
    mut reader: impl Read,
    limit: u64,
    observed: &AtomicU64,
    overflow: &AtomicBool,
    done: &AtomicBool,
    retain: bool,
) -> Result<Drained, RoutineError> {
    let mut retained = Vec::new();
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        if done.load(Ordering::Acquire) && overflow.load(Ordering::Acquire) {
            return Ok(Drained {
                retained,
                sha256: format!("sha256:{:x}", hasher.finalize()),
                closed: false,
            });
        }
        let read = match reader.read(&mut buffer) {
            Ok(read) => read,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if done.load(Ordering::Acquire) {
                    return Ok(Drained {
                        retained,
                        sha256: format!("sha256:{:x}", hasher.finalize()),
                        closed: false,
                    });
                }
                std::thread::sleep(Duration::from_millis(2));
                continue;
            }
            Err(_) => return Err(mediator_error("mediator-output-read-failed")),
        };
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        let prior = observed.fetch_add(read as u64, Ordering::AcqRel);
        let remaining = limit.saturating_sub(prior);
        let accepted = remaining.min(read as u64) as usize;
        if retain {
            retained.extend_from_slice(&buffer[..accepted]);
        }
        if accepted != read {
            overflow.store(true, Ordering::Release);
        }
    }
    Ok(Drained {
        retained,
        sha256: format!("sha256:{:x}", hasher.finalize()),
        closed: true,
    })
}

#[cfg(unix)]
fn set_nonblocking(file: &impl AsRawFd) -> Result<(), RoutineError> {
    let descriptor = file.as_raw_fd();
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(descriptor, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
    {
        return Err(mediator_error("mediator-output-nonblocking-failed"));
    }
    Ok(())
}

#[cfg(test)]
type SetupFailureHook = Box<dyn FnOnce() + Send + 'static>;

#[cfg(test)]
fn setup_failure_hook() -> &'static Mutex<Option<(SetupFailurePoint, SetupFailureHook)>> {
    static HOOK: OnceLock<Mutex<Option<(SetupFailurePoint, SetupFailureHook)>>> = OnceLock::new();
    HOOK.get_or_init(|| Mutex::new(None))
}

#[cfg(test)]
pub(crate) fn set_test_process_setup_failure(
    point: SetupFailurePoint,
    hook: impl FnOnce() + Send + 'static,
) {
    *setup_failure_hook()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some((point, Box::new(hook)));
}

#[cfg(test)]
fn maybe_inject_setup_failure(
    point: SetupFailurePoint,
    cause: &'static str,
) -> Result<(), RoutineError> {
    let hook = {
        let mut slot = setup_failure_hook()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if slot.as_ref().map(|(expected, _)| *expected) == Some(point) {
            slot.take().map(|(_, hook)| hook)
        } else {
            None
        }
    };
    if let Some(hook) = hook {
        hook();
        return Err(mediator_error(cause));
    }
    Ok(())
}

#[cfg(not(test))]
fn maybe_inject_setup_failure(
    _point: SetupFailurePoint,
    _cause: &'static str,
) -> Result<(), RoutineError> {
    Ok(())
}

#[cfg(target_os = "macos")]
fn sandbox_profile(
    program: &Path,
    working_directory: &Path,
    read_sources: &[&Path],
    scopes: &[&Path],
) -> Result<String, RoutineError> {
    let mut profile = String::from(
        "(version 1)\n(allow default)\n(deny network*)\n(deny process-fork (with send-signal SIGKILL))\n(deny process-exec)\n(deny file-map-executable)\n(allow file-map-executable (subpath \"/System\"))\n(allow file-map-executable (subpath \"/usr/lib\"))\n(deny file-read*)\n(allow file-read* (literal \"/\"))\n(allow file-read* (subpath \"/System\"))\n(allow file-read* (subpath \"/usr/lib\"))\n(allow file-read* (subpath \"/private/var/db/dyld\"))\n(deny file-write*)\n(deny file-clone file-link)\n",
    );
    let program = program
        .to_str()
        .ok_or_else(|| mediator_error("mediator-executable-path-not-utf8"))?;
    profile.push_str("(allow process-exec (literal \"");
    profile.push_str(&sandbox_escape(program)?);
    profile.push_str("\"))\n");
    for ancestor in working_directory
        .ancestors()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        if ancestor == Path::new("/") {
            continue;
        }
        let text = ancestor
            .to_str()
            .ok_or_else(|| mediator_error("mediator-working-directory-not-utf8"))?;
        profile.push_str("(allow file-read-metadata (literal \"");
        profile.push_str(&sandbox_escape(text)?);
        profile.push_str("\"))\n");
    }
    profile.push_str("(allow file-read* (literal \"");
    profile.push_str(&sandbox_escape(program)?);
    profile.push_str("\"))\n");
    for source in read_sources {
        let text = source
            .to_str()
            .ok_or_else(|| mediator_error("mediator-read-source-path-not-utf8"))?;
        profile.push_str("(allow file-read* (literal \"");
        profile.push_str(&sandbox_escape(text)?);
        profile.push_str("\"))\n");
        if let Some(alias) = private_alias(source) {
            let alias = alias
                .to_str()
                .ok_or_else(|| mediator_error("mediator-read-source-path-not-utf8"))?;
            profile.push_str("(allow file-read* (literal \"");
            profile.push_str(&sandbox_escape(alias)?);
            profile.push_str("\"))\n");
        }
    }
    for scope in scopes {
        let text = scope
            .to_str()
            .ok_or_else(|| mediator_error("mediator-output-scope-not-utf8"))?;
        for ancestor in scope.ancestors().collect::<Vec<_>>().into_iter().rev() {
            if ancestor == Path::new("/") || ancestor == working_directory {
                continue;
            }
            let ancestor = ancestor
                .to_str()
                .ok_or_else(|| mediator_error("mediator-output-scope-not-utf8"))?;
            profile.push_str("(allow file-read-metadata (literal \"");
            profile.push_str(&sandbox_escape(ancestor)?);
            profile.push_str("\"))\n");
            if let Some(alias) = private_alias(Path::new(ancestor)) {
                let alias = alias
                    .to_str()
                    .ok_or_else(|| mediator_error("mediator-output-scope-not-utf8"))?;
                profile.push_str("(allow file-read-metadata (literal \"");
                profile.push_str(&sandbox_escape(alias)?);
                profile.push_str("\"))\n");
            }
        }
        profile.push_str("(allow file-read-metadata (subpath \"");
        profile.push_str(&sandbox_escape(text)?);
        profile.push_str("\"))\n");
        if let Some(alias) = private_alias(scope) {
            let alias = alias
                .to_str()
                .ok_or_else(|| mediator_error("mediator-output-scope-not-utf8"))?;
            profile.push_str("(allow file-read-metadata (subpath \"");
            profile.push_str(&sandbox_escape(alias)?);
            profile.push_str("\"))\n");
            profile.push_str("(allow file-write* (subpath \"");
            profile.push_str(&sandbox_escape(alias)?);
            profile.push_str("\"))\n");
        }
        profile.push_str("(allow file-write* (subpath \"");
        profile.push_str(&sandbox_escape(text)?);
        profile.push_str("\"))\n");
    }
    Ok(profile)
}

#[cfg(target_os = "macos")]
fn private_alias(path: &Path) -> Option<PathBuf> {
    path.strip_prefix("/private")
        .ok()
        .filter(|suffix| !suffix.as_os_str().is_empty())
        .map(|suffix| Path::new("/").join(suffix))
}

#[cfg(target_os = "macos")]
fn sandbox_escape(value: &str) -> Result<String, RoutineError> {
    if value.bytes().any(|byte| byte.is_ascii_control()) {
        return Err(mediator_error("mediator-sandbox-path-invalid"));
    }
    Ok(value.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(unix)]
fn status_kind(status: ExitStatus) -> ProcessTermination {
    status.code().map_or_else(
        || ProcessTermination::Signaled(status.signal().unwrap_or(0)),
        ProcessTermination::Exited,
    )
}

#[cfg(unix)]
fn process_group_exists(group: ProcessGroupId) -> Result<bool, RoutineError> {
    if unsafe { libc::kill(group.signal_target()?, 0) } == 0 {
        return Ok(true);
    }
    let error = std::io::Error::last_os_error();
    match error.raw_os_error() {
        Some(libc::ESRCH) => Ok(false),
        Some(libc::EPERM) => Ok(true),
        _ => Err(mediator_error("mediator-process-group-probe-failed")),
    }
}

#[cfg(unix)]
fn signal_group(group: ProcessGroupId, signal: i32) -> Result<(), RoutineError> {
    if unsafe { libc::kill(group.signal_target()?, signal) } == 0 {
        return Ok(());
    }
    let error = std::io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ESRCH) {
        Ok(())
    } else {
        Err(mediator_error("mediator-process-group-signal-failed"))
    }
}

#[cfg(unix)]
fn cleanup_spawned_child(
    child: &mut Child,
    process_group: Option<ProcessGroupId>,
) -> Result<(), RoutineError> {
    if let Some(process_group) = process_group {
        terminate_and_reap(child, process_group)
    } else {
        let _ = child.kill();
        child
            .wait()
            .map(|_| ())
            .map_err(|_| mediator_error("mediator-process-reap-failed"))
    }
}

#[cfg(unix)]
fn terminate_and_reap(
    child: &mut std::process::Child,
    group: ProcessGroupId,
) -> Result<(), RoutineError> {
    let mut first_error = None;
    if let Err(error) = signal_group(group, libc::SIGTERM) {
        first_error = Some(error);
    }
    let deadline = Instant::now() + Duration::from_millis(100);
    let mut parent_reaped = false;
    while Instant::now() < deadline {
        match child.try_wait() {
            Ok(Some(_)) => {
                parent_reaped = true;
                break;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(2)),
            Err(_) => {
                first_error
                    .get_or_insert_with(|| mediator_error("mediator-process-reap-status-failed"));
                break;
            }
        }
    }
    if !parent_reaped {
        if let Err(error) = signal_group(group, libc::SIGKILL) {
            first_error.get_or_insert(error);
        }
        let _ = child.kill();
        if child.wait().is_err() {
            first_error.get_or_insert_with(|| mediator_error("mediator-process-reap-failed"));
        }
    }
    match process_group_exists(group) {
        Ok(true) => {
            if let Err(error) = signal_group(group, libc::SIGKILL) {
                first_error.get_or_insert(error);
            }
        }
        Ok(false) => {}
        Err(error) => {
            first_error.get_or_insert(error);
            let _ = signal_group(group, libc::SIGKILL);
        }
    }
    if let Err(error) = wait_group_absent(group) {
        first_error.get_or_insert(error);
    }
    first_error.map_or(Ok(()), Err)
}

#[cfg(unix)]
fn terminate_group_after_parent_exit(group: ProcessGroupId) -> Result<(), RoutineError> {
    signal_group(group, libc::SIGTERM)?;
    let deadline = Instant::now() + Duration::from_millis(100);
    while Instant::now() < deadline {
        if !process_group_exists(group)? {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    signal_group(group, libc::SIGKILL)?;
    wait_group_absent(group)
}

#[cfg(unix)]
fn wait_group_absent(group: ProcessGroupId) -> Result<(), RoutineError> {
    let deadline = Instant::now() + Duration::from_secs(1);
    while Instant::now() < deadline {
        if !process_group_exists(group)? {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    Err(mediator_error("mediator-descendant-cleanup-incomplete"))
}

fn digest_bytes(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn mediator_error(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::ObservationFailed, cause, None)
}

#[cfg(test)]
mod tests {
    use super::ProcessGroupId;

    #[test]
    fn positive_pid_is_never_used_as_a_process_group_signal_target() {
        let group = ProcessGroupId::new(78_302).unwrap();
        assert_eq!(group.signal_target().unwrap(), -78_302);
        assert!(ProcessGroupId::new(0).is_err());
        assert!(ProcessGroupId::new(-1).is_err());
        assert!(ProcessGroupId::new(i32::MIN).is_err());
        assert!(ProcessGroupId::from_child_id(i32::MAX as u32 + 1).is_err());
    }
}

#[cfg(test)]
pub(crate) fn test_spawn_count() -> u64 {
    TEST_SPAWN_COUNT.load(Ordering::SeqCst)
}
