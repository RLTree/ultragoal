use super::darwin::DarwinSuspendedProcess;
use std::collections::BTreeMap;
use std::ffi::{CString, c_char};
use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

pub(super) fn spawn_suspended_descriptor(
    program: &Path,
    cwd: RawFd,
    argv: &[String],
    environment: &BTreeMap<String, String>,
) -> io::Result<DarwinSuspendedProcess> {
    let path = CString::new(program.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "program path"))?;
    let arguments = c_arguments(program, argv)?;
    let variables = c_environment(environment)?;
    let mut argument_pointers = pointers(&arguments);
    let mut variable_pointers = pointers(&variables);
    let stdin = PipePair::new()?;
    let stdout = PipePair::new()?;
    let stderr = PipePair::new()?;
    let actions = FileActions::new(cwd, &stdin, &stdout, &stderr)?;
    let attributes = SpawnAttributes::new()?;
    let mut pid = 0;
    // SAFETY: all C strings and pointer vectors remain live through the call,
    // actions and attributes were initialized, and pid is writable storage.
    let result = unsafe {
        libc::posix_spawn(
            &mut pid,
            path.as_ptr(),
            &actions.0,
            &attributes.0,
            argument_pointers.as_mut_ptr(),
            variable_pointers.as_mut_ptr(),
        )
    };
    if result != 0 || pid <= 0 {
        return Err(io::Error::from_raw_os_error(if result == 0 {
            libc::ESRCH
        } else {
            result
        }));
    }
    Ok(DarwinSuspendedProcess::from_spawn(
        pid,
        stdin.into_write(),
        stdout.into_read(),
        stderr.into_read(),
    ))
}

fn c_arguments(program: &Path, argv: &[String]) -> io::Result<Vec<CString>> {
    std::iter::once(program.as_os_str().as_bytes())
        .chain(argv.iter().skip(1).map(String::as_bytes))
        .map(|value| {
            CString::new(value).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "argument"))
        })
        .collect()
}

fn c_environment(environment: &BTreeMap<String, String>) -> io::Result<Vec<CString>> {
    environment
        .iter()
        .map(|(key, value)| {
            CString::new(format!("{key}={value}"))
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "environment"))
        })
        .collect()
}

fn pointers(values: &[CString]) -> Vec<*mut c_char> {
    values
        .iter()
        .map(|value| value.as_ptr().cast_mut())
        .chain(std::iter::once(std::ptr::null_mut()))
        .collect()
}

struct PipePair {
    read: File,
    write: File,
}

impl PipePair {
    fn new() -> io::Result<Self> {
        let mut descriptors = [-1; 2];
        // SAFETY: descriptors is writable storage for the two pipe descriptors.
        if unsafe { libc::pipe(descriptors.as_mut_ptr()) } != 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: successful pipe returns owned descriptors consumed once by File.
        let read = unsafe { File::from_raw_fd(descriptors[0]) };
        // SAFETY: successful pipe returns owned descriptors consumed once by File.
        let write = unsafe { File::from_raw_fd(descriptors[1]) };
        set_close_on_exec(&read)?;
        set_close_on_exec(&write)?;
        Ok(Self { read, write })
    }

    fn into_read(self) -> File {
        self.read
    }

    fn into_write(self) -> File {
        self.write
    }
}

fn set_close_on_exec(file: &File) -> io::Result<()> {
    let descriptor = file.as_raw_fd();
    // SAFETY: descriptor is borrowed from a live File for this fcntl call.
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFD) };
    // SAFETY: descriptor remains borrowed from the live File and flags came from F_GETFD.
    if flags < 0 || unsafe { libc::fcntl(descriptor, libc::F_SETFD, flags | libc::FD_CLOEXEC) } < 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

struct FileActions(libc::posix_spawn_file_actions_t);

impl FileActions {
    fn new(cwd: RawFd, stdin: &PipePair, stdout: &PipePair, stderr: &PipePair) -> io::Result<Self> {
        let mut value = std::ptr::null_mut();
        // SAFETY: value is writable storage for the file-actions initializer.
        if unsafe { libc::posix_spawn_file_actions_init(&mut value) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let mut actions = Self(value);
        // SAFETY: actions was initialized and cwd is the live root descriptor.
        if unsafe { posix_spawn_file_actions_addfchdir_np(&mut actions.0, cwd) } != 0
            || add_dup_and_close(&mut actions.0, stdin.read.as_raw_fd(), 0).is_err()
            || add_dup_and_close(&mut actions.0, stdout.write.as_raw_fd(), 1).is_err()
            || add_dup_and_close(&mut actions.0, stderr.write.as_raw_fd(), 2).is_err()
            // SAFETY: actions remains initialized and cwd is the descriptor recorded above.
            || unsafe { libc::posix_spawn_file_actions_addclose(&mut actions.0, cwd) } != 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(actions)
    }
}

fn add_dup_and_close(
    actions: &mut libc::posix_spawn_file_actions_t,
    source: RawFd,
    target: RawFd,
) -> Result<(), ()> {
    // SAFETY: actions is initialized and source/target are pipe or standard descriptors.
    if unsafe { libc::posix_spawn_file_actions_adddup2(actions, source, target) } != 0
        || (source != target
            // SAFETY: actions is initialized and source is recorded for this spawn action list.
            && unsafe { libc::posix_spawn_file_actions_addclose(actions, source) } != 0)
    {
        return Err(());
    }
    Ok(())
}

impl Drop for FileActions {
    fn drop(&mut self) {
        // SAFETY: the action list was initialized and is destroyed once.
        unsafe { libc::posix_spawn_file_actions_destroy(&mut self.0) };
    }
}

struct SpawnAttributes(libc::posix_spawnattr_t);

impl SpawnAttributes {
    fn new() -> io::Result<Self> {
        let mut value = std::ptr::null_mut();
        // SAFETY: value is writable storage for the attributes initializer.
        if unsafe { libc::posix_spawnattr_init(&mut value) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let mut attributes = Self(value);
        let flags = (libc::POSIX_SPAWN_SETPGROUP
            | libc::POSIX_SPAWN_START_SUSPENDED
            | libc::POSIX_SPAWN_CLOEXEC_DEFAULT) as i16;
        // SAFETY: attributes was initialized and process group 0 requests the child group.
        if unsafe { libc::posix_spawnattr_setpgroup(&mut attributes.0, 0) } != 0
            // SAFETY: attributes remains initialized and flags contain supported spawn options.
            || unsafe { libc::posix_spawnattr_setflags(&mut attributes.0, flags) } != 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(attributes)
    }
}

impl Drop for SpawnAttributes {
    fn drop(&mut self) {
        // SAFETY: attributes was initialized and is destroyed once.
        unsafe { libc::posix_spawnattr_destroy(&mut self.0) };
    }
}

#[link(name = "System")]
unsafe extern "C" {
    fn posix_spawn_file_actions_addfchdir_np(
        actions: *mut libc::posix_spawn_file_actions_t,
        descriptor: RawFd,
    ) -> i32;
}
