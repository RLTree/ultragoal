use super::*;

type ConfinedResult =
    Result<(Option<i32>, Vec<u8>, bool, Option<&'static str>), FixtureScheduleError>;
type ConfinedRunner = for<'a> fn(
    &'a FixtureSpec,
    &'a Path,
    PinnedExecutableKind,
    &'a [u8],
    &'a [std::ffi::OsString],
    &'a Path,
    &'a BTreeMap<String, String>,
    usize,
    &'a Arc<AtomicBool>,
) -> ConfinedResult;

pub(crate) const RUN_CONFINED: ConfinedRunner =
    |fixture,
     source_executable,
     executable_kind,
     executable_bytes,
     arguments,
     cwd,
     environment,
     output_limit,
     interrupt| {
        run_confined_inner(ConfinedExecution {
            fixture,
            source_executable,
            executable_kind,
            executable_bytes,
            arguments,
            cwd,
            environment,
            output_limit,
            interrupt,
        })
    };
pub(crate) use RUN_CONFINED as run_confined;

struct ConfinedExecution<'a> {
    fixture: &'a FixtureSpec,
    source_executable: &'a Path,
    executable_kind: PinnedExecutableKind,
    executable_bytes: &'a [u8],
    arguments: &'a [std::ffi::OsString],
    cwd: &'a Path,
    environment: &'a BTreeMap<String, String>,
    output_limit: usize,
    interrupt: &'a Arc<AtomicBool>,
}

fn run_confined_inner(request: ConfinedExecution<'_>) -> ConfinedResult {
    let ConfinedExecution {
        fixture,
        source_executable,
        executable_kind,
        executable_bytes,
        arguments,
        cwd,
        environment,
        output_limit,
        interrupt,
    } = request;
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
            super::super::super::environment::InvocationSensitivity::Public,
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
            // SAFETY: this closure runs only in the spawned child before exec;
            // `directory` is a live descriptor owned by `snapshot` until spawn.
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
pub(crate) struct TestNativeSnapshot {
    pub(crate) lease_root: std::fs::File,
    pub(crate) directory: std::fs::File,
    pub(crate) directory_name: std::ffi::CString,
}

#[cfg(all(test, unix))]
impl TestNativeSnapshot {
    pub(crate) fn create(cwd: &Path, bytes: &[u8]) -> Result<Self, FixtureScheduleError> {
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

pub(crate) fn digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("sha256:{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
pub(crate) static TEST_PRE_LAUNCH_PAUSED: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
pub(crate) fn pre_launch_hook() -> &'static std::sync::Mutex<Option<(std::path::PathBuf, u64)>> {
    static HOOK: std::sync::OnceLock<std::sync::Mutex<Option<(std::path::PathBuf, u64)>>> =
        std::sync::OnceLock::new();
    HOOK.get_or_init(|| std::sync::Mutex::new(None))
}
