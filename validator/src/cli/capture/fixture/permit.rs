use crate::fixture_scheduler::{FixtureScheduleError, FixtureSpec};
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

const MAX_OUTPUT: usize = 1024 * 1024;

#[cfg(unix)]
#[derive(Clone, Copy)]
pub(super) struct ExecutableIdentity {
    device: u64,
    inode: u64,
    length: u64,
    modified_seconds: i64,
    modified_nanos: i64,
}

/// Crate-internal permit binding a pinned invocation to immutable fixture
/// metadata.  Public `CommandSpec` cannot construct or replace this permit.
pub(crate) struct FixtureCaptureAdapter {
    pub(super) fixture_id: String,
    pub(super) fixture_digest: String,
    pub(super) executable: PathBuf,
    pub(super) executable_digest: String,
    #[cfg(unix)]
    pub(super) executable_identity: ExecutableIdentity,
    pub(super) arguments: Vec<OsString>,
    pub(super) output_limit: usize,
    pub(super) interrupt: Arc<AtomicBool>,
    pub(super) required_output: Vec<u8>,
}

impl FixtureCaptureAdapter {
    pub(crate) fn issue(
        fixture: &FixtureSpec,
        executable: PathBuf,
        arguments: Vec<OsString>,
        output_limit: usize,
        required_output: Vec<u8>,
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
        let metadata = std::fs::symlink_metadata(&executable)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if metadata.file_type().is_symlink()
                || !metadata.is_file()
                || metadata.nlink() != 1
                || metadata.mode() & 0o022 != 0
                || metadata.mode() & 0o111 == 0
            {
                return Err(FixtureScheduleError::Integrity(
                    "fixture executable identity is unsafe".to_owned(),
                ));
            }
        }
        #[cfg(not(unix))]
        if !metadata.is_file() {
            return Err(FixtureScheduleError::Integrity(
                "fixture executable is not a regular file".to_owned(),
            ));
        }
        let executable_digest =
            crate::digest::file(&executable).map_err(FixtureScheduleError::Integrity)?;
        Ok(Self {
            fixture_id: fixture.id.clone(),
            fixture_digest: fixture.metadata_digest.clone(),
            executable,
            executable_digest,
            #[cfg(unix)]
            executable_identity: ExecutableIdentity::from(&metadata),
            arguments,
            output_limit,
            interrupt: Arc::new(AtomicBool::new(false)),
            required_output,
        })
    }

    pub(super) fn validate_executable(&self) -> Result<(), FixtureScheduleError> {
        let metadata = std::fs::symlink_metadata(&self.executable)?;
        #[cfg(unix)]
        if ExecutableIdentity::from(&metadata) != self.executable_identity {
            return Err(FixtureScheduleError::Integrity(
                "fixture executable identity changed before launch".to_owned(),
            ));
        }
        let digest =
            crate::digest::file(&self.executable).map_err(FixtureScheduleError::Integrity)?;
        if digest != self.executable_digest {
            return Err(FixtureScheduleError::Integrity(
                "fixture executable content changed before launch".to_owned(),
            ));
        }
        Ok(())
    }

    pub(crate) fn interrupt(&self) {
        self.interrupt.store(true, Ordering::SeqCst);
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
            modified_seconds: metadata.mtime(),
            modified_nanos: metadata.mtime_nsec(),
        }
    }
}

#[cfg(unix)]
impl PartialEq for ExecutableIdentity {
    fn eq(&self, other: &Self) -> bool {
        self.device == other.device
            && self.inode == other.inode
            && self.length == other.length
            && self.modified_seconds == other.modified_seconds
            && self.modified_nanos == other.modified_nanos
    }
}
