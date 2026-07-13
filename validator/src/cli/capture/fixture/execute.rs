use super::super::output::{self, OutputBudget};
use super::permit::{FixtureCaptureAdapter, PinnedExecutableKind};
use crate::fixture_scheduler::{
    ExecutedFixture, ExpectedOutcome, FixtureExecutionRecord, FixtureExecutor,
    FixtureScheduleError, FixtureSpec, IsolationLease, ObservedOutcome, OutcomeVerdict,
    RecordedFixtureExecutor,
};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

impl FixtureExecutor for FixtureCaptureAdapter {
    fn execute(
        &self,
        fixture: &FixtureSpec,
        lease: &IsolationLease,
        environment: &BTreeMap<String, String>,
    ) -> Result<ObservedOutcome, FixtureScheduleError> {
        self.capture(fixture, lease, environment)
            .map(|executed| executed.observed)
    }
}

impl RecordedFixtureExecutor for FixtureCaptureAdapter {
    fn execute_recorded(
        &self,
        fixture: &FixtureSpec,
        lease: &IsolationLease,
        environment: &BTreeMap<String, String>,
    ) -> Result<ExecutedFixture, FixtureScheduleError> {
        self.capture(fixture, lease, environment)
    }
}

impl FixtureCaptureAdapter {
    fn capture(
        &self,
        fixture: &FixtureSpec,
        lease: &IsolationLease,
        environment: &BTreeMap<String, String>,
    ) -> Result<ExecutedFixture, FixtureScheduleError> {
        if fixture.id != self.fixture_id || fixture.metadata_digest != self.fixture_digest {
            return Err(FixtureScheduleError::Integrity(
                "fixture execution permit does not bind this immutable specification".to_owned(),
            ));
        }
        let executable_bytes = self.execution_bytes()?;
        #[cfg(unix)]
        match self.executable_kind {
            PinnedExecutableKind::ProtectedNative => self.validate_protected_native()?,
            PinnedExecutableKind::PosixShellSource => Self::validate_posix_shell_substrate()?,
            #[cfg(test)]
            PinnedExecutableKind::TestNativeSnapshot => {}
        }
        let (exit, stdout, overflow, early_termination) = run_confined(
            fixture,
            &self.executable,
            self.executable_kind,
            &executable_bytes,
            &self.arguments,
            lease.root(),
            environment,
            self.output_limit,
            &self.interrupt,
        )?;
        let output_redacted = sensitive_text(&stdout);
        let stdout_digest = digest(if output_redacted {
            b"<redacted>"
        } else {
            &stdout
        });
        // Stderr remains subject to the shared bounded-output budget but is
        // not retained by this adapter, so its canonical digest is empty.
        let stderr_digest = digest(b"");
        let artifact = match self.artifact_name.as_deref() {
            Some(name) => read_artifact(lease.root(), name, self.output_limit)?,
            None => stdout.clone(),
        };
        if sensitive_text(&artifact) {
            return Err(FixtureScheduleError::Integrity(
                "fixture artifact contains secret-shaped data".to_owned(),
            ));
        }
        let expected = &fixture.expected;
        let valid_exit = matches!(expected.verdict, OutcomeVerdict::Pass) == (exit == Some(0));
        let observed = if overflow {
            ObservedOutcome::failure("fixture-output-limit-exceeded", 0)
        } else if let Some(causal_code) = early_termination {
            ObservedOutcome::failure(causal_code, 0)
        } else if !valid_exit || stdout != self.required_output {
            ObservedOutcome::failure("fixture-observation-mismatch", 0)
        } else {
            observed_from(expected)
        };
        let artifact_relative_path = self
            .artifact_name
            .as_ref()
            .map(|name| format!("file/{name}"))
            .unwrap_or_else(|| "captured-output.bin".to_owned());
        let record = FixtureExecutionRecord::captured(
            self.binding.clone(),
            fixture.id.clone(),
            fixture.metadata_digest.clone(),
            lease.id().to_owned(),
            self.executable_digest.clone(),
            self.executable_identity_sha256(),
            artifact_relative_path,
            artifact,
            observed.clone(),
            exit,
            stdout_digest,
            stderr_digest,
            output_redacted,
        );
        Ok(ExecutedFixture { observed, record })
    }

    #[cfg(test)]
    pub(crate) fn set_test_pre_launch_pause(path: std::path::PathBuf, milliseconds: u64) {
        TEST_PRE_LAUNCH_PAUSED.store(false, Ordering::SeqCst);
        *pre_launch_hook()
            .lock()
            .expect("fixture pre-launch hook lock") = Some((path, milliseconds));
    }

    #[cfg(test)]
    pub(crate) fn test_pre_launch_is_paused() -> bool {
        TEST_PRE_LAUNCH_PAUSED.load(Ordering::SeqCst)
    }
}

fn observed_from(expected: &ExpectedOutcome) -> ObservedOutcome {
    match expected.verdict {
        OutcomeVerdict::Pass => ObservedOutcome::pass(expected.maximum_claim_ceiling),
        OutcomeVerdict::Fail => {
            ObservedOutcome::failure(&expected.causal_code, expected.maximum_claim_ceiling)
        }
        OutcomeVerdict::Quarantined => ObservedOutcome::quarantined(&expected.causal_code),
    }
}

fn run_confined(
    fixture: &FixtureSpec,
    source_executable: &Path,
    executable_kind: PinnedExecutableKind,
    executable_bytes: &[u8],
    arguments: &[std::ffi::OsString],
    cwd: &Path,
    environment: &BTreeMap<String, String>,
    output_limit: usize,
    interrupt: &Arc<AtomicBool>,
) -> Result<(Option<i32>, Vec<u8>, bool, Option<&'static str>), FixtureScheduleError> {
    #[cfg(not(unix))]
    {
        let _ = (
            source_executable,
            executable_kind,
            executable_bytes,
            arguments,
            cwd,
            environment,
            output_limit,
            interrupt,
        );
        return Err(FixtureScheduleError::Integrity(
            "fixture execution requires Unix process groups".to_owned(),
        ));
    }
    #[cfg(unix)]
    {
        let budget = Arc::new(OutputBudget::for_sensitivity(
            output_limit,
            super::super::environment::InvocationSensitivity::Public,
        ));
        let plan = crate::fixture_scheduler::ConfinementPlan::prepare(&fixture.confinement, cwd)?;
        #[cfg(test)]
        let mut test_snapshot = None;
        let (launch_executable, launch_arguments) = match executable_kind {
            PinnedExecutableKind::ProtectedNative => {
                (source_executable.to_path_buf(), arguments.to_vec())
            }
            PinnedExecutableKind::PosixShellSource => {
                use std::os::unix::ffi::OsStringExt;

                let mut shell_arguments = Vec::with_capacity(arguments.len().saturating_add(3));
                shell_arguments.push(std::ffi::OsString::from("-c"));
                shell_arguments.push(std::ffi::OsString::from_vec(executable_bytes.to_vec()));
                shell_arguments.push(source_executable.as_os_str().to_os_string());
                shell_arguments.extend(arguments.iter().cloned());
                (std::path::PathBuf::from("/bin/sh"), shell_arguments)
            }
            #[cfg(test)]
            PinnedExecutableKind::TestNativeSnapshot => {
                let snapshot = TestNativeSnapshot::create(cwd, executable_bytes)?;
                test_snapshot = Some(snapshot);
                (std::path::PathBuf::from("./fixture"), arguments.to_vec())
            }
        };
        let mut command = plan.command(&launch_executable, &launch_arguments, cwd, environment)?;
        #[cfg(test)]
        if let Some(snapshot) = test_snapshot.as_ref() {
            use std::os::fd::AsRawFd;
            use std::os::unix::process::CommandExt;

            let directory = snapshot.directory.as_raw_fd();
            unsafe {
                command.pre_exec(move || {
                    if libc::fchdir(directory) != 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
        }
        test_pre_launch_pause(source_executable);
        let started = Instant::now();
        let mut child = command.spawn().map_err(FixtureScheduleError::Io)?;
        let stdout = child.stdout.take().ok_or_else(|| {
            FixtureScheduleError::Integrity("fixture stdout unavailable".to_owned())
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            FixtureScheduleError::Integrity("fixture stderr unavailable".to_owned())
        })?;
        let left = Arc::clone(&budget);
        let right = Arc::clone(&budget);
        let reader = std::thread::spawn(move || output::observe(stdout, output_limit, &left));
        let err_reader = std::thread::spawn(move || output::observe(stderr, output_limit, &right));
        let (status, early_termination) = loop {
            if interrupt.load(Ordering::SeqCst) {
                terminate_and_reap(&mut child)?;
                break (None, Some("fixture-interrupted"));
            }
            if budget.exceeded() {
                terminate_and_reap(&mut child)?;
                break (None, Some("fixture-output-limit-exceeded"));
            }
            if started.elapsed() >= fixture.confinement.wall_time() {
                terminate_and_reap(&mut child)?;
                break (None, Some("fixture-wall-time-exceeded"));
            }
            match child.try_wait().map_err(FixtureScheduleError::Io)? {
                Some(status) => break (status.code(), None),
                None => std::thread::sleep(Duration::from_millis(2)),
            }
        };
        let out = reader
            .join()
            .map_err(|_| {
                FixtureScheduleError::Integrity("fixture stdout reader panicked".to_owned())
            })?
            .map_err(FixtureScheduleError::Integrity)?;
        let err = err_reader
            .join()
            .map_err(|_| {
                FixtureScheduleError::Integrity("fixture stderr reader panicked".to_owned())
            })?
            .map_err(FixtureScheduleError::Integrity)?;
        let outputs = budget.finalize_streams(out, err);
        Ok((
            status,
            outputs.first.retained().to_vec(),
            outputs.output_limit_exceeded,
            early_termination,
        ))
    }
}

#[cfg(all(test, unix))]
struct TestNativeSnapshot {
    lease_root: std::fs::File,
    directory: std::fs::File,
    directory_name: std::ffi::CString,
}

#[cfg(all(test, unix))]
impl TestNativeSnapshot {
    fn create(cwd: &Path, bytes: &[u8]) -> Result<Self, FixtureScheduleError> {
        use std::ffi::CString;
        use std::io::Write;
        use std::os::fd::FromRawFd;
        use std::os::unix::ffi::OsStrExt;

        let cwd = CString::new(cwd.as_os_str().as_bytes()).map_err(|_| {
            FixtureScheduleError::InvalidMetadata("fixture lease root contains NUL".to_owned())
        })?;
        let root_descriptor = unsafe {
            libc::open(
                cwd.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if root_descriptor < 0 {
            return Err(FixtureScheduleError::Io(std::io::Error::last_os_error()));
        }
        let lease_root = unsafe { std::fs::File::from_raw_fd(root_descriptor) };
        let mut random = [0_u8; 16];
        getrandom::fill(&mut random).map_err(|_| {
            FixtureScheduleError::Integrity("test snapshot randomness unavailable".to_owned())
        })?;
        let directory_name = CString::new(format!(
            ".fixture-native-{}",
            random
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ))
        .expect("hex snapshot name contains no NUL");
        if unsafe { libc::mkdirat(root_descriptor, directory_name.as_ptr(), 0o700) } != 0 {
            return Err(FixtureScheduleError::Io(std::io::Error::last_os_error()));
        }
        let directory_descriptor = unsafe {
            libc::openat(
                root_descriptor,
                directory_name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if directory_descriptor < 0 {
            unsafe {
                libc::unlinkat(root_descriptor, directory_name.as_ptr(), libc::AT_REMOVEDIR);
            }
            return Err(FixtureScheduleError::Io(std::io::Error::last_os_error()));
        }
        let directory = unsafe { std::fs::File::from_raw_fd(directory_descriptor) };
        let snapshot = Self {
            lease_root,
            directory,
            directory_name,
        };
        let file_name = c"fixture";
        let file_descriptor = unsafe {
            libc::openat(
                directory_descriptor,
                file_name.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o500,
            )
        };
        if file_descriptor < 0 {
            return Err(FixtureScheduleError::Io(std::io::Error::last_os_error()));
        }
        let mut file = unsafe { std::fs::File::from_raw_fd(file_descriptor) };
        file.write_all(bytes)?;
        file.sync_all()?;
        if unsafe { libc::fchmod(file_descriptor, 0o500) } != 0
            || unsafe { libc::fchmod(directory_descriptor, 0o500) } != 0
        {
            return Err(FixtureScheduleError::Io(std::io::Error::last_os_error()));
        }
        snapshot.lease_root.sync_all()?;
        Ok(snapshot)
    }
}

#[cfg(all(test, unix))]
impl Drop for TestNativeSnapshot {
    fn drop(&mut self) {
        use std::os::fd::AsRawFd;

        unsafe {
            libc::fchmod(self.directory.as_raw_fd(), 0o700);
            libc::unlinkat(self.directory.as_raw_fd(), c"fixture".as_ptr(), 0);
            libc::unlinkat(
                self.lease_root.as_raw_fd(),
                self.directory_name.as_ptr(),
                libc::AT_REMOVEDIR,
            );
        }
    }
}

fn digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("sha256:{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
static TEST_PRE_LAUNCH_PAUSED: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
fn pre_launch_hook() -> &'static std::sync::Mutex<Option<(std::path::PathBuf, u64)>> {
    static HOOK: std::sync::OnceLock<std::sync::Mutex<Option<(std::path::PathBuf, u64)>>> =
        std::sync::OnceLock::new();
    HOOK.get_or_init(|| std::sync::Mutex::new(None))
}

#[cfg(test)]
fn test_pre_launch_pause(path: &Path) {
    let milliseconds = {
        let mut hook = pre_launch_hook()
            .lock()
            .expect("fixture pre-launch hook lock");
        if hook.as_ref().is_some_and(|(target, _)| target == path) {
            hook.take().map(|(_, milliseconds)| milliseconds)
        } else {
            None
        }
    };
    if let Some(milliseconds) = milliseconds {
        TEST_PRE_LAUNCH_PAUSED.store(true, Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(milliseconds));
    }
}

#[cfg(not(test))]
fn test_pre_launch_pause(_path: &Path) {}

fn sensitive_text(bytes: &[u8]) -> bool {
    let text = String::from_utf8_lossy(bytes).to_ascii_lowercase();
    [
        "api_key",
        "api-key",
        "authorization:",
        "bearer ",
        "private_key",
        "client_secret",
        "access_token",
        "secret=",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

#[cfg(unix)]
fn read_artifact(root: &Path, name: &str, limit: usize) -> Result<Vec<u8>, FixtureScheduleError> {
    use std::ffi::CString;
    use std::io::Read;
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

    let directory = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW)
        .open(root.join("file"))?;
    let name = CString::new(name).map_err(|_| {
        FixtureScheduleError::InvalidMetadata("evaluation artifact name".to_owned())
    })?;
    let descriptor = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
        )
    };
    if descriptor < 0 {
        return Err(FixtureScheduleError::Io(std::io::Error::last_os_error()));
    }
    let mut file = unsafe { std::fs::File::from_raw_fd(descriptor) };
    let before = file.metadata()?;
    if !before.is_file()
        || before.nlink() != 1
        || before.mode() & 0o022 != 0
        || before.len() == 0
        || before.len() > limit as u64
    {
        return Err(FixtureScheduleError::Integrity(
            "fixture artifact identity is unsafe".to_owned(),
        ));
    }
    let mut bytes = Vec::with_capacity(before.len() as usize);
    file.by_ref()
        .take((limit as u64).saturating_add(1))
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 != before.len() || bytes.len() > limit {
        return Err(FixtureScheduleError::Integrity(
            "fixture artifact changed or exceeded its bound".to_owned(),
        ));
    }
    let after = file.metadata()?;
    let mut current = std::mem::MaybeUninit::<libc::stat>::uninit();
    if unsafe {
        libc::fstatat(
            directory.as_raw_fd(),
            name.as_ptr(),
            current.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    } != 0
    {
        return Err(FixtureScheduleError::Io(std::io::Error::last_os_error()));
    }
    let current = unsafe { current.assume_init() };
    if before.dev() != after.dev()
        || before.ino() != after.ino()
        || before.len() != after.len()
        || before.mtime() != after.mtime()
        || before.mtime_nsec() != after.mtime_nsec()
        || before.dev() != current.st_dev as u64
        || before.ino() != current.st_ino as u64
        || current.st_mode & libc::S_IFMT != libc::S_IFREG
        || current.st_nlink != 1
    {
        return Err(FixtureScheduleError::Integrity(
            "fixture artifact changed during capture".to_owned(),
        ));
    }
    Ok(bytes)
}

#[cfg(not(unix))]
fn read_artifact(
    _root: &Path,
    _name: &str,
    _limit: usize,
) -> Result<Vec<u8>, FixtureScheduleError> {
    Err(FixtureScheduleError::Integrity(
        "fixture artifact capture requires Unix descriptor confinement".to_owned(),
    ))
}

#[cfg(unix)]
fn terminate_and_reap(child: &mut std::process::Child) -> Result<(), FixtureScheduleError> {
    let process = i32::try_from(child.id())
        .map_err(|_| FixtureScheduleError::Integrity("fixture process id is invalid".to_owned()))?;
    let group = process.checked_neg().ok_or_else(|| {
        FixtureScheduleError::Integrity("fixture process group id is invalid".to_owned())
    })?;
    signal_process_group(group, libc::SIGTERM)?;
    signal_process(process, libc::SIGTERM)?;
    let deadline = Instant::now() + Duration::from_millis(50);
    let mut reaped = false;
    while Instant::now() < deadline {
        if child
            .try_wait()
            .map_err(FixtureScheduleError::Io)?
            .is_some()
        {
            reaped = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    let reap_error = if reaped {
        None
    } else {
        signal_process_group(group, libc::SIGKILL)?;
        signal_process(process, libc::SIGKILL)?;
        child.wait().err()
    };
    wait_for_process_group_absence(group)?;
    if let Some(error) = reap_error {
        return Err(FixtureScheduleError::Io(error));
    }
    Ok(())
}

#[cfg(unix)]
fn signal_process(process: i32, signal: i32) -> Result<(), FixtureScheduleError> {
    if unsafe { libc::kill(process, signal) } == 0 {
        return Ok(());
    }
    let error = std::io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ESRCH) {
        return Ok(());
    }
    Err(FixtureScheduleError::Integrity(format!(
        "fixture process signal {signal} failed: {error}"
    )))
}

#[cfg(unix)]
fn signal_process_group(group: i32, signal: i32) -> Result<(), FixtureScheduleError> {
    if unsafe { libc::kill(group, signal) } == 0 {
        return Ok(());
    }
    let error = std::io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ESRCH) {
        return Ok(());
    }
    Err(FixtureScheduleError::Integrity(format!(
        "fixture process group signal {signal} failed: {error}"
    )))
}

#[cfg(unix)]
fn wait_for_process_group_absence(group: i32) -> Result<(), FixtureScheduleError> {
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        if unsafe { libc::kill(group, 0) } != 0 {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::ESRCH) {
                return Ok(());
            }
            if error.raw_os_error() != Some(libc::EPERM) {
                return Err(FixtureScheduleError::Integrity(format!(
                    "fixture process group absence probe failed: {error}"
                )));
            }
        }
        if Instant::now() >= deadline {
            return Err(FixtureScheduleError::Integrity(
                "fixture process group remained after SIGKILL".to_owned(),
            ));
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}
