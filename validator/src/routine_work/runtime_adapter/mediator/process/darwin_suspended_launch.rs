use super::*;
use std::ffi::{CString, c_char};
use std::fs::File;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;

pub(crate) struct SpawnedProcess {
    pub(crate) child: BoundChild,
    pub(crate) stdin: File,
    pub(crate) stdout: File,
    pub(crate) stderr: File,
}

pub(crate) fn spawn_suspended(
    program: &PinnedExecutable,
    root: &RootAnchor,
    argv: &[String],
    environment: &BTreeMap<String, String>,
) -> Result<SpawnedProcess, RoutineError> {
    let path = CString::new(program.path().as_os_str().as_bytes())
        .map_err(|_| mediator_error("mediator-executable-path-invalid"))?;
    let arguments = c_arguments(program.path(), argv)?;
    let variables = c_environment(environment)?;
    let mut argument_pointers = pointers(&arguments);
    let mut variable_pointers = pointers(&variables);
    let stdin = PipePair::new()?;
    let stdout = PipePair::new()?;
    let stderr = PipePair::new()?;
    let actions = FileActions::new(root.raw_fd(), &stdin, &stdout, &stderr)?;
    let attributes = SpawnAttributes::new()?;
    let mut pid = 0;
    // SAFETY: all C strings and pointer vectors remain live through the call, actions and
    // attributes were initialized, and pid is writable process-id storage.
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
        return Err(mediator_error("mediator-process-launch-failed"));
    }
    Ok(SpawnedProcess {
        child: BoundChild { pid, status: None },
        stdin: stdin.into_write(),
        stdout: stdout.into_read(),
        stderr: stderr.into_read(),
    })
}

fn c_arguments(program: &Path, argv: &[String]) -> Result<Vec<CString>, RoutineError> {
    std::iter::once(program.as_os_str().as_bytes())
        .chain(argv.iter().skip(1).map(String::as_bytes))
        .map(|value| CString::new(value).map_err(|_| mediator_error("mediator-argv-invalid")))
        .collect()
}

fn c_environment(environment: &BTreeMap<String, String>) -> Result<Vec<CString>, RoutineError> {
    environment
        .iter()
        .map(|(key, value)| {
            CString::new(format!("{key}={value}"))
                .map_err(|_| mediator_error("mediator-environment-invalid"))
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
    fn new() -> Result<Self, RoutineError> {
        let mut descriptors = [-1; 2];
        // SAFETY: descriptors is writable storage for the two pipe descriptors.
        if unsafe { libc::pipe(descriptors.as_mut_ptr()) } != 0 {
            return Err(mediator_error("mediator-pipe-create-failed"));
        }
        // SAFETY: successful pipe returns an owned read descriptor consumed once by File.
        let read = unsafe { File::from_raw_fd(descriptors[0]) };
        // SAFETY: successful pipe returns an owned write descriptor consumed once by File.
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

fn set_close_on_exec(file: &File) -> Result<(), RoutineError> {
    let descriptor = file.as_raw_fd();
    // SAFETY: descriptor is borrowed from a live File for this fcntl call.
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFD) };
    // SAFETY: descriptor remains borrowed from the live File and flags came from F_GETFD.
    if flags < 0 || unsafe { libc::fcntl(descriptor, libc::F_SETFD, flags | libc::FD_CLOEXEC) } < 0
    {
        return Err(mediator_error("mediator-pipe-cloexec-failed"));
    }
    Ok(())
}

struct FileActions(libc::posix_spawn_file_actions_t);

impl FileActions {
    fn new(
        cwd: i32,
        stdin: &PipePair,
        stdout: &PipePair,
        stderr: &PipePair,
    ) -> Result<Self, RoutineError> {
        let mut value = std::ptr::null_mut();
        // SAFETY: value is writable storage for the platform file-actions initializer.
        if unsafe { libc::posix_spawn_file_actions_init(&mut value) } != 0 {
            return Err(mediator_error("mediator-spawn-actions-invalid"));
        }
        let mut actions = Self(value);
        // SAFETY: actions was initialized and cwd is the live root directory descriptor.
        if unsafe { posix_spawn_file_actions_addfchdir_np(&mut actions.0, cwd) } != 0
            || add_dup_and_close(&mut actions.0, stdin.read.as_raw_fd(), 0).is_err()
            || add_dup_and_close(&mut actions.0, stdout.write.as_raw_fd(), 1).is_err()
            || add_dup_and_close(&mut actions.0, stderr.write.as_raw_fd(), 2).is_err()
            // SAFETY: actions remains initialized and cwd is the descriptor recorded above.
            || unsafe { libc::posix_spawn_file_actions_addclose(&mut actions.0, cwd) } != 0
        {
            return Err(mediator_error("mediator-spawn-actions-invalid"));
        }
        Ok(actions)
    }
}

fn add_dup_and_close(
    actions: &mut libc::posix_spawn_file_actions_t,
    source: i32,
    target: i32,
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
        // SAFETY: FileActions contains a successfully initialized action list and destroys it once.
        unsafe { libc::posix_spawn_file_actions_destroy(&mut self.0) };
    }
}

struct SpawnAttributes(libc::posix_spawnattr_t);

impl SpawnAttributes {
    fn new() -> Result<Self, RoutineError> {
        let mut value = std::ptr::null_mut();
        // SAFETY: value is writable storage for the platform spawn-attributes initializer.
        if unsafe { libc::posix_spawnattr_init(&mut value) } != 0 {
            return Err(mediator_error("mediator-spawn-attributes-invalid"));
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
            return Err(mediator_error("mediator-spawn-attributes-invalid"));
        }
        Ok(attributes)
    }
}

impl Drop for SpawnAttributes {
    fn drop(&mut self) {
        // SAFETY: SpawnAttributes contains initialized platform attributes and destroys them once.
        unsafe { libc::posix_spawnattr_destroy(&mut self.0) };
    }
}

#[link(name = "System")]
unsafe extern "C" {
    fn posix_spawn_file_actions_addfchdir_np(
        actions: *mut libc::posix_spawn_file_actions_t,
        descriptor: i32,
    ) -> i32;
}
