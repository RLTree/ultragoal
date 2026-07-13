use crate::fixture_scheduler::{
    FixtureExecutionBinding, FixtureScheduleError, FixtureSpec, ResourceKind,
};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

const MAX_OUTPUT: usize = 1024 * 1024;
const MAX_EXECUTABLE_BYTES: usize = 64 * 1024 * 1024;
const MAX_SHELL_SOURCE_BYTES: usize = 64 * 1024;
#[cfg(unix)]
const PROTECTED_NATIVE_PATHS: [&str; 2] = ["/usr/bin/false", "/usr/bin/true"];
#[cfg(unix)]
const POSIX_SHELL_PATH: &str = "/bin/sh";

#[cfg(unix)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ExecutableIdentity {
    device: u64,
    inode: u64,
    length: u64,
    link_count: u64,
    user_id: u32,
    group_id: u32,
    mode: u32,
    modified_seconds: i64,
    modified_nanos: i64,
    changed_seconds: i64,
    changed_nanos: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PinnedExecutableKind {
    ProtectedNative,
    PosixShellSource,
    #[cfg(test)]
    TestNativeSnapshot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExecutableIssuancePolicy {
    Production,
    ShellSubstrate,
    #[cfg(test)]
    TestNativeSnapshot,
}

/// Crate-internal permit binding a pinned invocation to immutable fixture
/// metadata.  Public `CommandSpec` cannot construct or replace this permit.
pub(crate) struct FixtureCaptureAdapter {
    pub(super) fixture_id: String,
    pub(super) fixture_digest: String,
    pub(super) executable: PathBuf,
    pub(super) executable_digest: String,
    executable_bytes: Arc<[u8]>,
    pub(super) executable_kind: PinnedExecutableKind,
    #[cfg(unix)]
    pub(super) executable_identity: ExecutableIdentity,
    pub(super) arguments: Vec<OsString>,
    pub(super) output_limit: usize,
    pub(super) interrupt: Arc<AtomicBool>,
    pub(super) required_output: Vec<u8>,
    pub(super) binding: FixtureExecutionBinding,
    pub(super) artifact_name: Option<String>,
}

impl FixtureCaptureAdapter {
    pub(crate) fn issue(
        fixture: &FixtureSpec,
        executable: PathBuf,
        arguments: Vec<OsString>,
        output_limit: usize,
        required_output: Vec<u8>,
    ) -> Result<Self, FixtureScheduleError> {
        let binding = FixtureExecutionBinding::standalone(&fixture.id, &fixture.metadata_digest);
        Self::issue_bound(
            fixture,
            executable,
            arguments,
            output_limit,
            required_output,
            binding,
            None,
            ExecutableIssuancePolicy::Production,
        )
    }

    #[cfg(test)]
    pub(crate) fn issue_test_native(
        fixture: &FixtureSpec,
        executable: PathBuf,
        arguments: Vec<OsString>,
        output_limit: usize,
        required_output: Vec<u8>,
    ) -> Result<Self, FixtureScheduleError> {
        let binding = FixtureExecutionBinding::standalone(&fixture.id, &fixture.metadata_digest);
        Self::issue_bound(
            fixture,
            executable,
            arguments,
            output_limit,
            required_output,
            binding,
            None,
            ExecutableIssuancePolicy::TestNativeSnapshot,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn issue_evaluation(
        fixture: &FixtureSpec,
        executable: PathBuf,
        arguments: Vec<OsString>,
        output_limit: usize,
        required_output: Vec<u8>,
        binding: FixtureExecutionBinding,
        artifact_name: impl Into<String>,
    ) -> Result<Self, FixtureScheduleError> {
        let artifact_name = artifact_name.into();
        if artifact_name.is_empty()
            || artifact_name.len() > 120
            || artifact_name == "."
            || artifact_name == ".."
            || artifact_name.contains('/')
            || artifact_name.contains('\\')
            || artifact_name.chars().any(char::is_control)
            || !fixture.resources.contains(&ResourceKind::File)
        {
            return Err(FixtureScheduleError::InvalidMetadata(
                "invalid evaluation artifact name".to_owned(),
            ));
        }
        Self::issue_bound(
            fixture,
            executable,
            arguments,
            output_limit,
            required_output,
            binding,
            Some(artifact_name),
            ExecutableIssuancePolicy::Production,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn issue_bound(
        fixture: &FixtureSpec,
        executable: PathBuf,
        arguments: Vec<OsString>,
        output_limit: usize,
        required_output: Vec<u8>,
        binding: FixtureExecutionBinding,
        artifact_name: Option<String>,
        issuance_policy: ExecutableIssuancePolicy,
    ) -> Result<Self, FixtureScheduleError> {
        fixture.validate()?;
        if !executable.is_absolute()
            || executable
                .components()
                .any(|part| matches!(part, std::path::Component::ParentDir))
            || !(1..=MAX_OUTPUT).contains(&output_limit)
            || required_output.len() > MAX_OUTPUT
        {
            return Err(FixtureScheduleError::InvalidMetadata(
                "invalid fixture capture permit".to_owned(),
            ));
        }
        #[cfg(not(unix))]
        {
            return Err(FixtureScheduleError::Integrity(
                "fixture executable pinning requires Unix".to_owned(),
            ));
        }
        #[cfg(unix)]
        {
            let (executable_bytes, executable_digest, executable_identity, executable_kind) =
                pin_source_executable(&executable, issuance_policy)?;
            Ok(Self {
                fixture_id: fixture.id.clone(),
                fixture_digest: fixture.metadata_digest.clone(),
                executable,
                executable_digest,
                executable_bytes: Arc::from(executable_bytes),
                executable_kind,
                executable_identity,
                arguments,
                output_limit,
                interrupt: Arc::new(AtomicBool::new(false)),
                required_output,
                binding,
                artifact_name,
            })
        }
    }

    pub(super) fn execution_bytes(&self) -> Result<Arc<[u8]>, FixtureScheduleError> {
        if crate::digest::bytes(&self.executable_bytes) != self.executable_digest {
            return Err(FixtureScheduleError::Integrity(
                "fixture executable snapshot binding changed before launch".to_owned(),
            ));
        }
        Ok(Arc::clone(&self.executable_bytes))
    }

    #[cfg(unix)]
    pub(super) fn validate_protected_native(&self) -> Result<(), FixtureScheduleError> {
        if self.executable_kind != PinnedExecutableKind::ProtectedNative {
            return Err(FixtureScheduleError::Integrity(
                "fixture executable is not a protected native substrate".to_owned(),
            ));
        }
        require_protected_native_path(&self.executable)?;
        let path_metadata = std::fs::symlink_metadata(&self.executable)?;
        require_safe_source(&path_metadata)?;
        if ExecutableIdentity::from(&path_metadata).user_id != 0
            || ExecutableIdentity::from(&path_metadata) != self.executable_identity
        {
            return Err(FixtureScheduleError::Integrity(
                "protected fixture executable identity changed".to_owned(),
            ));
        }
        let file = open_source_descriptor(&self.executable)?;
        if ExecutableIdentity::from(&file.metadata()?) != self.executable_identity
            || crate::digest::bytes(&read_exact_descriptor(&file, MAX_EXECUTABLE_BYTES)?)
                != self.executable_digest
        {
            return Err(FixtureScheduleError::Integrity(
                "protected fixture executable content changed".to_owned(),
            ));
        }
        Ok(())
    }

    #[cfg(unix)]
    pub(super) fn validate_posix_shell_substrate() -> Result<(), FixtureScheduleError> {
        let (_, _, identity, kind) = pin_source_executable(
            Path::new(POSIX_SHELL_PATH),
            ExecutableIssuancePolicy::ShellSubstrate,
        )?;
        if identity.user_id != 0 || kind != PinnedExecutableKind::ProtectedNative {
            return Err(FixtureScheduleError::Integrity(
                "fixed POSIX shell substrate is not protected".to_owned(),
            ));
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn set_test_issue_pause(path: PathBuf, milliseconds: u64) {
        TEST_ISSUE_PAUSED.store(false, Ordering::SeqCst);
        *issue_hook().lock().expect("fixture issue hook lock") = Some((path, milliseconds));
    }

    #[cfg(test)]
    pub(crate) fn test_issue_is_paused() -> bool {
        TEST_ISSUE_PAUSED.load(Ordering::SeqCst)
    }

    pub(crate) fn interrupt(&self) {
        self.interrupt.store(true, Ordering::SeqCst);
    }

    #[cfg(unix)]
    pub(super) fn executable_identity_sha256(&self) -> String {
        crate::fixture_scheduler::stable_digest(&format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.executable_identity.device,
            self.executable_identity.inode,
            self.executable_identity.length,
            self.executable_identity.link_count,
            self.executable_identity.user_id,
            self.executable_identity.group_id,
            self.executable_identity.mode,
            self.executable_identity.modified_seconds,
            self.executable_identity.modified_nanos,
            self.executable_identity.changed_seconds,
            self.executable_identity.changed_nanos,
        ))
    }

    #[cfg(not(unix))]
    pub(super) fn executable_identity_sha256(&self) -> String {
        self.executable_digest.clone()
    }
}

#[cfg(unix)]
impl ExecutableIdentity {
    fn from(metadata: &std::fs::Metadata) -> Self {
        use std::os::unix::fs::MetadataExt;
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            length: metadata.len(),
            link_count: metadata.nlink(),
            user_id: metadata.uid(),
            group_id: metadata.gid(),
            mode: metadata.mode(),
            modified_seconds: metadata.mtime(),
            modified_nanos: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanos: metadata.ctime_nsec(),
        }
    }
}

#[cfg(unix)]
fn pin_source_executable(
    path: &Path,
    issuance_policy: ExecutableIssuancePolicy,
) -> Result<(Vec<u8>, String, ExecutableIdentity, PinnedExecutableKind), FixtureScheduleError> {
    let path_before = std::fs::symlink_metadata(path)?;
    require_safe_source(&path_before)?;
    let file = open_source_descriptor(path)?;
    let opened = file.metadata()?;
    require_safe_source(&opened)?;
    let identity = ExecutableIdentity::from(&opened);
    if ExecutableIdentity::from(&path_before) != identity {
        return Err(FixtureScheduleError::Integrity(
            "fixture executable changed while it was opened".to_owned(),
        ));
    }
    let bytes = read_exact_descriptor(&file, MAX_EXECUTABLE_BYTES)?;
    test_issue_pause(path);
    let descriptor_after = file.metadata()?;
    let path_after = std::fs::symlink_metadata(path)?;
    require_safe_source(&descriptor_after)?;
    require_safe_source(&path_after)?;
    if ExecutableIdentity::from(&descriptor_after) != identity
        || ExecutableIdentity::from(&path_after) != identity
    {
        return Err(FixtureScheduleError::Integrity(
            "fixture executable changed during permit issuance".to_owned(),
        ));
    }
    let digest = crate::digest::bytes(&bytes);
    let kind = classify_executable(path, &bytes, identity, issuance_policy)?;
    Ok((bytes, digest, identity, kind))
}

#[cfg(unix)]
fn classify_executable(
    path: &Path,
    bytes: &[u8],
    identity: ExecutableIdentity,
    _issuance_policy: ExecutableIssuancePolicy,
) -> Result<PinnedExecutableKind, FixtureScheduleError> {
    let native_magic = bytes.get(..4).is_some_and(|magic| {
        matches!(
            magic,
            b"\x7fELF"
                | b"\xfe\xed\xfa\xce"
                | b"\xfe\xed\xfa\xcf"
                | b"\xce\xfa\xed\xfe"
                | b"\xcf\xfa\xed\xfe"
                | b"\xca\xfe\xba\xbe"
                | b"\xca\xfe\xba\xbf"
        )
    });
    if native_magic && identity.user_id == 0 {
        if _issuance_policy == ExecutableIssuancePolicy::ShellSubstrate {
            require_protected_path(path, &[POSIX_SHELL_PATH])?;
        } else {
            require_protected_native_path(path)?;
        }
        return Ok(PinnedExecutableKind::ProtectedNative);
    }
    #[cfg(test)]
    if native_magic && _issuance_policy == ExecutableIssuancePolicy::TestNativeSnapshot {
        return Ok(PinnedExecutableKind::TestNativeSnapshot);
    }
    if bytes.len() <= MAX_SHELL_SOURCE_BYTES
        && bytes.starts_with(b"#!/bin/sh\n")
        && !bytes.contains(&0)
    {
        return Ok(PinnedExecutableKind::PosixShellSource);
    }
    Err(FixtureScheduleError::Integrity(
        "fixture executable must be a protected native binary or exact POSIX shell source"
            .to_owned(),
    ))
}

#[cfg(unix)]
fn require_protected_native_path(path: &Path) -> Result<(), FixtureScheduleError> {
    require_protected_path(path, &PROTECTED_NATIVE_PATHS)
}

#[cfg(unix)]
fn require_protected_path(path: &Path, allowed_paths: &[&str]) -> Result<(), FixtureScheduleError> {
    use std::os::unix::fs::MetadataExt;

    if !allowed_paths
        .iter()
        .any(|allowed| path == Path::new(allowed))
    {
        return Err(FixtureScheduleError::Integrity(
            "fixture native executable is not a fixed protected substrate".to_owned(),
        ));
    }

    let mut component = Some(path);
    while let Some(current) = component {
        let metadata = std::fs::symlink_metadata(current)?;
        if metadata.file_type().is_symlink()
            || metadata.uid() != 0
            || metadata.mode() & 0o022 != 0
            || (current == path
                && (!metadata.is_file() || metadata.nlink() != 1 || metadata.mode() & 0o111 == 0))
            || (current != path && !metadata.is_dir())
        {
            return Err(FixtureScheduleError::Integrity(
                "fixture native executable path authority is unsafe".to_owned(),
            ));
        }
        component = current.parent().filter(|parent| *parent != current);
    }
    Ok(())
}

#[cfg(unix)]
fn open_source_descriptor(path: &Path) -> Result<std::fs::File, FixtureScheduleError> {
    use std::ffi::CString;
    use std::os::fd::FromRawFd;
    use std::os::unix::ffi::OsStrExt;

    let path = CString::new(path.as_os_str().as_bytes()).map_err(|_| {
        FixtureScheduleError::InvalidMetadata("fixture executable path contains NUL".to_owned())
    })?;
    let descriptor = unsafe {
        libc::open(
            path.as_ptr(),
            libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
        )
    };
    if descriptor < 0 {
        return Err(FixtureScheduleError::Io(std::io::Error::last_os_error()));
    }
    Ok(unsafe { std::fs::File::from_raw_fd(descriptor) })
}

#[cfg(unix)]
fn require_safe_source(metadata: &std::fs::Metadata) -> Result<(), FixtureScheduleError> {
    use std::os::unix::fs::MetadataExt;

    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.nlink() != 1
        || metadata.len() == 0
        || metadata.len() > MAX_EXECUTABLE_BYTES as u64
        || metadata.mode() & 0o022 != 0
        || metadata.mode() & 0o111 == 0
    {
        return Err(FixtureScheduleError::Integrity(
            "fixture executable identity is unsafe".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn read_exact_descriptor(
    file: &std::fs::File,
    maximum_bytes: usize,
) -> Result<Vec<u8>, FixtureScheduleError> {
    use std::os::unix::fs::FileExt;

    let before = file.metadata()?;
    if before.len() == 0 || before.len() > maximum_bytes as u64 {
        return Err(FixtureScheduleError::Integrity(
            "fixture executable size is out of bounds".to_owned(),
        ));
    }
    let identity = ExecutableIdentity::from(&before);
    let mut bytes = vec![0_u8; before.len() as usize];
    let mut offset = 0_usize;
    while offset < bytes.len() {
        let read = file.read_at(&mut bytes[offset..], offset as u64)?;
        if read == 0 {
            return Err(FixtureScheduleError::Integrity(
                "fixture executable shortened during snapshot".to_owned(),
            ));
        }
        offset += read;
    }
    if ExecutableIdentity::from(&file.metadata()?) != identity {
        return Err(FixtureScheduleError::Integrity(
            "fixture executable changed during snapshot".to_owned(),
        ));
    }
    Ok(bytes)
}

#[cfg(test)]
static TEST_ISSUE_PAUSED: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
fn issue_hook() -> &'static std::sync::Mutex<Option<(PathBuf, u64)>> {
    static HOOK: std::sync::OnceLock<std::sync::Mutex<Option<(PathBuf, u64)>>> =
        std::sync::OnceLock::new();
    HOOK.get_or_init(|| std::sync::Mutex::new(None))
}

#[cfg(test)]
fn test_issue_pause(path: &Path) {
    let milliseconds = {
        let mut hook = issue_hook().lock().expect("fixture issue hook lock");
        if hook.as_ref().is_some_and(|(target, _)| target == path) {
            hook.take().map(|(_, milliseconds)| milliseconds)
        } else {
            None
        }
    };
    if let Some(milliseconds) = milliseconds {
        TEST_ISSUE_PAUSED.store(true, Ordering::SeqCst);
        std::thread::sleep(std::time::Duration::from_millis(milliseconds));
    }
}

#[cfg(not(test))]
fn test_issue_pause(_path: &Path) {}
