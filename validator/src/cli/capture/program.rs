#[cfg(unix)]
use super::descriptor::Snapshot;
use super::descriptor::read_descriptor;
use super::util::{digest_bytes, is_prohibited_component};
use crate::context::LiveContext;
use std::ffi::CString;
use std::fs::File;
use std::path::{Component, Path, PathBuf};

#[cfg(unix)]
use std::os::fd::FromRawFd;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

const MAX_PROGRAM_BYTES: usize = 64 * 1024 * 1024;

/// A root-owned native tool captured by `LiveContext` and pinned for one run.
pub(super) struct PinnedProgram {
    path: PathBuf,
    file: File,
    #[cfg(unix)]
    snapshot: Snapshot,
    sha256: String,
}

impl PinnedProgram {
    pub fn open(context: &LiveContext, capability: &str) -> Result<Self, String> {
        let tool = context
            .capabilities()
            .tool(capability)
            .filter(|tool| tool.available)
            .ok_or_else(|| "capture program capability is unavailable".to_owned())?;
        let path = PathBuf::from(
            tool.executable
                .as_deref()
                .ok_or_else(|| "capture program capability has no executable".to_owned())?,
        );
        validate_path(&path)?;
        let file = open_no_follow(&path)?;
        let metadata = secure_metadata(&file)?;
        let (bytes, sha256) = read_descriptor(&file, MAX_PROGRAM_BYTES)?;
        let expected = tool
            .executable_sha256
            .as_deref()
            .ok_or_else(|| "capture program capability has no digest".to_owned())?;
        if sha256.strip_prefix("sha256:") != Some(expected)
            || tool.byte_length != Some(bytes.len() as u64)
            || !native_executable(&bytes)
        {
            return Err("capture program capability identity is unsupported".to_owned());
        }
        Ok(Self {
            path,
            file,
            #[cfg(unix)]
            snapshot: Snapshot::from(&metadata),
            sha256,
        })
    }

    pub fn validate(&self) -> Result<(), String> {
        let descriptor = secure_metadata(&self.file)?;
        let current = open_no_follow(&self.path)?;
        let current_metadata = secure_metadata(&current)?;
        #[cfg(unix)]
        if Snapshot::from(&descriptor) != self.snapshot
            || Snapshot::from(&current_metadata) != self.snapshot
        {
            return Err("capture program identity changed".to_owned());
        }
        let (_, digest) = read_descriptor(&current, MAX_PROGRAM_BYTES)?;
        if digest != self.sha256 {
            return Err("capture program content changed".to_owned());
        }
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn sha256(&self) -> &str {
        &self.sha256
    }
}

fn validate_path(path: &Path) -> Result<(), String> {
    if !path.is_absolute()
        || path.components().any(|component| {
            matches!(component, Component::Normal(value) if is_prohibited_component(value))
        })
    {
        return Err("capture program capability path is unsafe".to_owned());
    }
    Ok(())
}

#[cfg(unix)]
fn open_no_follow(path: &Path) -> Result<File, String> {
    let path = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| "capture program capability path contains NUL".to_owned())?;
    let fd = unsafe {
        libc::open(
            path.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err("capture program capability open failed".to_owned());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}

#[cfg(not(unix))]
fn open_no_follow(_path: &Path) -> Result<File, String> {
    Err("capture program pinning requires Unix".to_owned())
}

#[cfg(unix)]
fn secure_metadata(file: &File) -> Result<std::fs::Metadata, String> {
    let metadata = file
        .metadata()
        .map_err(|_| "capture program capability metadata failed".to_owned())?;
    if !metadata.is_file()
        || metadata.nlink() != 1
        || metadata.uid() != 0
        || metadata.mode() & 0o111 == 0
        || metadata.mode() & 0o022 != 0
    {
        return Err("capture program capability is not a protected native file".to_owned());
    }
    Ok(metadata)
}

#[cfg(not(unix))]
fn secure_metadata(_file: &File) -> Result<std::fs::Metadata, String> {
    Err("capture program pinning requires Unix".to_owned())
}

fn native_executable(bytes: &[u8]) -> bool {
    let Some(magic) = bytes.get(..4) else {
        return false;
    };
    matches!(
        magic,
        b"\x7fELF"
            | b"\xfe\xed\xfa\xce"
            | b"\xfe\xed\xfa\xcf"
            | b"\xce\xfa\xed\xfe"
            | b"\xcf\xfa\xed\xfe"
            | b"\xca\xfe\xba\xbe"
            | b"\xca\xfe\xba\xbf"
    ) && digest_bytes(bytes) != digest_bytes(&[])
}
