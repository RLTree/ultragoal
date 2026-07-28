use super::pipes::Pipes;
use super::sandbox;
use crate::distribution::HostCommand;
use std::ffi::{CString, c_char};
use std::fs::File;
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::path::Path;

pub(super) struct Spawned {
    pub(super) pid: libc::pid_t,
    pub(super) pipes: Pipes,
}

pub(super) fn spawn(path: &Path, command: &HostCommand, cwd: RawFd) -> Result<Spawned, ()> {
    let cwd = duplicate_cwd(cwd)?;
    let arguments = sandbox::arguments(path, command)?;
    let mut argv = pointers(&arguments);
    let environment = command
        .environment()
        .iter()
        .map(|(key, value)| CString::new(format!("{key}={value}")))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| ())?;
    let mut envp = pointers(&environment);
    let mut pipes = Pipes::new().map_err(|_| ())?;
    let actions = FileActions::new(cwd.as_raw_fd(), &pipes)?;
    let attributes = SpawnAttributes::new()?;
    let executable = sandbox::verified_executable()?;
    let mut pid = 0;
    // SAFETY: all strings and pointer vectors remain live through the call;
    // actions and attributes were initialized, and pid is writable storage.
    let result = unsafe {
        libc::posix_spawn(
            &mut pid,
            executable.as_ptr(),
            &actions.0,
            &attributes.0,
            argv.as_mut_ptr(),
            envp.as_mut_ptr(),
        )
    };
    if result != 0 || pid <= 0 {
        pipes.close_all();
        return Err(());
    }
    Ok(Spawned { pid, pipes })
}

fn duplicate_cwd(cwd: RawFd) -> Result<File, ()> {
    // SAFETY: cwd is borrowed for this operation; fcntl returns a new owned fd.
    let duplicate = unsafe { libc::fcntl(cwd, libc::F_DUPFD_CLOEXEC, 0) };
    if duplicate < 0 {
        return Err(());
    }
    // SAFETY: duplicate is a fresh owned descriptor from F_DUPFD_CLOEXEC.
    let file = unsafe { File::from_raw_fd(duplicate) };
    file.metadata()
        .map_err(|_| ())?
        .is_dir()
        .then_some(file)
        .ok_or(())
}

fn pointers(values: &[CString]) -> Vec<*mut c_char> {
    values
        .iter()
        .map(|value| value.as_ptr().cast_mut())
        .chain(std::iter::once(std::ptr::null_mut()))
        .collect()
}

struct FileActions(libc::posix_spawn_file_actions_t);

impl FileActions {
    fn new(cwd: RawFd, pipes: &Pipes) -> Result<Self, ()> {
        let mut value = std::ptr::null_mut();
        // SAFETY: value is writable storage for the initializer.
        if unsafe { libc::posix_spawn_file_actions_init(&mut value) } != 0 {
            return Err(());
        }
        let mut actions = Self(value);
        // SAFETY: actions is initialized and cwd is the owned duplicate.
        if unsafe { posix_spawn_file_actions_addfchdir_np(&mut actions.0, cwd) } != 0
            || add_dup_and_close(&mut actions.0, pipes.stdin_read(), libc::STDIN_FILENO).is_err()
            || add_dup_and_close(&mut actions.0, pipes.stdout_write(), libc::STDOUT_FILENO).is_err()
            || add_dup_and_close(&mut actions.0, pipes.stderr_write(), libc::STDERR_FILENO).is_err()
            // SAFETY: actions remains initialized and cwd is the recorded descriptor.
            || unsafe { libc::posix_spawn_file_actions_addclose(&mut actions.0, cwd) } != 0
        {
            return Err(());
        }
        Ok(actions)
    }
}

impl Drop for FileActions {
    fn drop(&mut self) {
        // SAFETY: the action list was initialized and is destroyed once.
        unsafe { libc::posix_spawn_file_actions_destroy(&mut self.0) };
    }
}

fn add_dup_and_close(
    actions: &mut libc::posix_spawn_file_actions_t,
    source: RawFd,
    target: RawFd,
) -> Result<(), ()> {
    // SAFETY: actions is initialized and source/target are live pipe descriptors.
    if unsafe { libc::posix_spawn_file_actions_adddup2(actions, source, target) } != 0
        || (source != target
            // SAFETY: actions is initialized and source is recorded for this action list.
            && unsafe { libc::posix_spawn_file_actions_addclose(actions, source) } != 0)
    {
        return Err(());
    }
    Ok(())
}

struct SpawnAttributes(libc::posix_spawnattr_t);

impl SpawnAttributes {
    fn new() -> Result<Self, ()> {
        let mut value = std::ptr::null_mut();
        // SAFETY: value is writable storage for the initializer.
        if unsafe { libc::posix_spawnattr_init(&mut value) } != 0 {
            return Err(());
        }
        let mut attributes = Self(value);
        let flags = (libc::POSIX_SPAWN_SETPGROUP
            | libc::POSIX_SPAWN_START_SUSPENDED
            | libc::POSIX_SPAWN_CLOEXEC_DEFAULT) as i16;
        // SAFETY: attributes is initialized and process group 0 requests the child group.
        if unsafe { libc::posix_spawnattr_setpgroup(&mut attributes.0, 0) } != 0
            // SAFETY: attributes remains initialized and flags are supported spawn options.
            || unsafe { libc::posix_spawnattr_setflags(&mut attributes.0, flags) } != 0
        {
            return Err(());
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
