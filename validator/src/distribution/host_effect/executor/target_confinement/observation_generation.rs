fn observation_generation(identity: &ObjectIdentity) -> u64 {
    let mut value = identity.inode
        ^ identity.device.rotate_left(7)
        ^ (identity.changed_seconds.max(0) as u64).rotate_left(17)
        ^ (identity.changed_nanoseconds.max(0) as u64).rotate_left(29);
    if value == 0 {
        value = 1;
    }
    value
}

fn open_directory_path(path: &Path) -> Result<File, HostEffectExecutorFailure> {
    let path = CString::new(path.as_os_str().as_bytes()).map_err(|_| io_failure())?;
    let descriptor = unsafe {
        libc::open(
            path.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if descriptor < 0 {
        return Err(io_failure());
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

fn stat_at(
    directory: libc::c_int,
    name: &CStr,
    flags: libc::c_int,
) -> Result<Option<Metadata>, HostEffectExecutorFailure> {
    let descriptor = unsafe {
        libc::openat(
            directory,
            name.as_ptr(),
            libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
        )
    };
    if descriptor >= 0 {
        let file = unsafe { File::from_raw_fd(descriptor) };
        return file.metadata().map(Some).map_err(|_| io_failure());
    }
    if last_errno() == Some(libc::ENOENT) {
        return Ok(None);
    }
    // O_NOFOLLOW intentionally refuses symlinks. fstatat observes their kind
    // without following them so callers can classify rather than mistake the
    // refusal for absence.
    let mut value = std::mem::MaybeUninit::<libc::stat>::uninit();
    let result = unsafe { libc::fstatat(directory, name.as_ptr(), value.as_mut_ptr(), flags) };
    if result != 0 {
        return if last_errno() == Some(libc::ENOENT) {
            Ok(None)
        } else {
            Err(io_failure())
        };
    }
    let value = unsafe { value.assume_init() };
    // Rust cannot construct Metadata from stat. Re-opening special objects can
    // block or activate device behavior, so a successfully observed
    // non-openable object is conservatively rejected as unsafe.
    let _ = value;
    Err(unsafe_object())
}

fn component(value: &str) -> Result<CString, HostEffectExecutorFailure> {
    if !valid_component(value) {
        return Err(unsafe_object());
    }
    CString::new(value).map_err(|_| unsafe_object())
}

fn valid_component(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 255
        && !matches!(value, "." | "..")
        && value.is_ascii()
        && !value.bytes().any(|byte| {
            byte == b'/'
                || byte == b'\\'
                || byte == 0
                || byte.is_ascii_control()
                || !(byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        })
}

#[cfg(target_os = "macos")]
fn rename_noreplace(directory: libc::c_int, old: &CStr, new: &CStr) -> libc::c_int {
    const RENAME_NOFOLLOW_ANY: libc::c_uint = 0x10;
    const RENAME_RESOLVE_BENEATH: libc::c_uint = 0x20;
    unsafe {
        libc::renameatx_np(
            directory,
            old.as_ptr(),
            directory,
            new.as_ptr(),
            libc::RENAME_EXCL | RENAME_NOFOLLOW_ANY | RENAME_RESOLVE_BENEATH,
        )
    }
}

#[cfg(target_os = "linux")]
fn rename_noreplace(directory: libc::c_int, old: &CStr, new: &CStr) -> libc::c_int {
    unsafe {
        libc::renameat2(
            directory,
            old.as_ptr(),
            directory,
            new.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn rename_noreplace(_directory: libc::c_int, _old: &CStr, _new: &CStr) -> libc::c_int {
    -1
}

fn last_errno() -> Option<i32> {
    std::io::Error::last_os_error().raw_os_error()
}

#[cfg(target_os = "linux")]
fn clear_errno() {
    unsafe {
        *libc::__errno_location() = 0;
    }
}

#[cfg(any(target_os = "macos", target_os = "freebsd"))]
fn clear_errno() {
    unsafe {
        *libc::__error() = 0;
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "freebsd")))]
fn clear_errno() {}

fn io_failure() -> HostEffectExecutorFailure {
    HostEffectExecutorFailure::new(HostEffectExecutorErrorId::Io)
}

fn path_swap() -> HostEffectExecutorFailure {
    HostEffectExecutorFailure::new(HostEffectExecutorErrorId::PathSwap)
}

fn target_substitution() -> HostEffectExecutorFailure {
    HostEffectExecutorFailure::new(HostEffectExecutorErrorId::TargetSubstitution)
}

fn unsafe_object() -> HostEffectExecutorFailure {
    HostEffectExecutorFailure::new(HostEffectExecutorErrorId::UnsafeObject)
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum FaultPoint {
    BeforeTempCreate,
    BeforeTempWrite,
    BeforeTempFsync,
    BeforeRename,
    BeforeDirectoryFsync,
}

#[cfg(not(test))]
#[derive(Clone, Copy)]
enum FaultPoint {
    BeforeTempCreate,
    BeforeTempWrite,
    BeforeTempFsync,
    BeforeRename,
    BeforeDirectoryFsync,
}

#[cfg(test)]
thread_local! {
    static FAULT: std::cell::RefCell<Option<(FaultPoint, bool, Box<dyn FnOnce(&Path)>)>> =
        std::cell::RefCell::new(None);
}

#[cfg(test)]
pub(super) fn set_fault(
    point: FaultPoint,
    force_failure: bool,
    hook: impl FnOnce(&Path) + 'static,
) {
    FAULT.with(|slot| {
        assert!(
            slot.borrow_mut()
                .replace((point, force_failure, Box::new(hook)))
                .is_none(),
            "only one executor fault hook may be armed per thread"
        );
    });
}
