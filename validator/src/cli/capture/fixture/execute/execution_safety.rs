use super::*;

#[cfg(test)]
pub(crate) fn test_pre_launch_pause(path: &Path) {
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
pub(crate) fn test_pre_launch_pause(_path: &Path) {}

pub(crate) fn sensitive_text(bytes: &[u8]) -> bool {
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
pub(crate) fn read_artifact(
    root: &Path,
    name: &str,
    limit: usize,
) -> Result<Vec<u8>, FixtureScheduleError> {
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
pub(crate) fn read_artifact(
    _root: &Path,
    _name: &str,
    _limit: usize,
) -> Result<Vec<u8>, FixtureScheduleError> {
    Err(FixtureScheduleError::Integrity(
        "fixture artifact capture requires Unix descriptor confinement".to_owned(),
    ))
}

#[cfg(unix)]
pub(crate) fn terminate_and_reap(
    child: &mut std::process::Child,
) -> Result<(), FixtureScheduleError> {
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
pub(crate) fn signal_process(process: i32, signal: i32) -> Result<(), FixtureScheduleError> {
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
pub(crate) fn signal_process_group(group: i32, signal: i32) -> Result<(), FixtureScheduleError> {
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
pub(crate) fn wait_for_process_group_absence(group: i32) -> Result<(), FixtureScheduleError> {
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
