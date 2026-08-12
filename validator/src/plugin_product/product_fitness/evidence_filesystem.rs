use super::super::model::ProductFitnessError;
use std::ffi::CString;
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Component, Path, PathBuf};

const MAX_EVIDENCE_BYTES: u64 = 64 * 1024 * 1024;
type RootIdentity = (u64, u64, u32);

pub(super) fn read_stable(root: &Path, value: &str) -> Result<Vec<u8>, ProductFitnessError> {
    #[cfg(not(unix))]
    {
        let _ = (root, value);
        return Err(ProductFitnessError::EvidencePathInvalid);
    }
    #[cfg(unix)]
    {
        let root = CheckedRoot::open(root)?;
        let relative = checked_relative(value)?;
        let mut file = root.open_relative(&relative)?;
        let before = file
            .metadata()
            .map_err(|_| ProductFitnessError::EvidenceMissing)?;
        validate_file(&before)?;
        let mut bytes = Vec::new();
        file.by_ref()
            .take(MAX_EVIDENCE_BYTES.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|_| ProductFitnessError::EvidenceMissing)?;
        if bytes.len() as u64 > MAX_EVIDENCE_BYTES || bytes.len() as u64 != before.len() {
            return Err(ProductFitnessError::EvidenceSpecialFile);
        }
        let after = file
            .metadata()
            .map_err(|_| ProductFitnessError::EvidenceMissing)?;
        let current = root
            .open_relative(&relative)
            .map_err(|_| ProductFitnessError::EvidenceStale)?
            .metadata()
            .map_err(|_| ProductFitnessError::EvidenceMissing)?;
        if file_identity(&before) != file_identity(&after)
            || file_identity(&before) != file_identity(&current)
        {
            return Err(ProductFitnessError::EvidenceStale);
        }
        Ok(bytes)
    }
}

struct CheckedRoot {
    file: File,
    identity: RootIdentity,
}

impl CheckedRoot {
    fn open(root: &Path) -> Result<Self, ProductFitnessError> {
        let before =
            fs::symlink_metadata(root).map_err(|_| ProductFitnessError::EvidencePathInvalid)?;
        if before.file_type().is_symlink() || !before.is_dir() {
            return Err(ProductFitnessError::EvidencePathInvalid);
        }
        let canonical =
            fs::canonicalize(root).map_err(|_| ProductFitnessError::EvidencePathInvalid)?;
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&canonical)
            .map_err(|_| ProductFitnessError::EvidencePathInvalid)?;
        let opened = file
            .metadata()
            .map_err(|_| ProductFitnessError::EvidencePathInvalid)?;
        let current = fs::symlink_metadata(&canonical)
            .map_err(|_| ProductFitnessError::EvidencePathInvalid)?;
        let identity = root_identity(&before);
        if identity != root_identity(&opened)
            || identity != root_identity(&current)
            || current.file_type().is_symlink()
            || !opened.is_dir()
        {
            return Err(ProductFitnessError::EvidenceStale);
        }
        Ok(Self { file, identity })
    }

    fn open_relative(&self, relative: &Path) -> Result<File, ProductFitnessError> {
        if root_identity(
            &self
                .file
                .metadata()
                .map_err(|_| ProductFitnessError::EvidenceStale)?,
        ) != self.identity
        {
            return Err(ProductFitnessError::EvidenceStale);
        }
        let mut directory = self
            .file
            .try_clone()
            .map_err(|_| ProductFitnessError::EvidencePathInvalid)?;
        let components = relative.components().collect::<Vec<_>>();
        for component in &components[..components.len().saturating_sub(1)] {
            let Component::Normal(name) = component else {
                return Err(ProductFitnessError::EvidencePathInvalid);
            };
            directory = open_at(
                &directory,
                name.as_bytes(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
            .map_err(|_| ProductFitnessError::EvidencePathInvalid)?;
        }
        let Some(Component::Normal(name)) = components.last() else {
            return Err(ProductFitnessError::EvidencePathInvalid);
        };
        open_at(
            &directory,
            name.as_bytes(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        )
        .map_err(|error| {
            if error.raw_os_error() == Some(libc::ELOOP) {
                ProductFitnessError::EvidenceSpecialFile
            } else {
                ProductFitnessError::EvidenceMissing
            }
        })
    }
}

fn open_at(directory: &File, name: &[u8], flags: i32) -> std::io::Result<File> {
    let name = CString::new(name)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid path"))?;
    // SAFETY: directory is live, name is one NUL-terminated component, and a
    // successful openat returns one newly owned descriptor.
    let descriptor = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
    if descriptor < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        // SAFETY: the successful descriptor is newly owned.
        Ok(unsafe { File::from_raw_fd(descriptor) })
    }
}

fn validate_file(metadata: &fs::Metadata) -> Result<(), ProductFitnessError> {
    if !metadata.is_file() || metadata.len() > MAX_EVIDENCE_BYTES || metadata.nlink() != 1 {
        return Err(ProductFitnessError::EvidenceSpecialFile);
    }
    Ok(())
}

fn checked_relative(value: &str) -> Result<PathBuf, ProductFitnessError> {
    if value.is_empty() || value.len() > 4096 || value.contains('\\') {
        return Err(ProductFitnessError::EvidencePathInvalid);
    }
    let path = Path::new(value);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(ProductFitnessError::EvidencePathInvalid);
    }
    Ok(path.to_path_buf())
}

fn root_identity(metadata: &fs::Metadata) -> RootIdentity {
    (metadata.dev(), metadata.ino(), metadata.mode())
}

fn file_identity(metadata: &fs::Metadata) -> (u64, u64, u64, u64, i64, i64, i64, i64) {
    (
        metadata.dev(),
        metadata.ino(),
        metadata.nlink(),
        metadata.len(),
        metadata.mtime(),
        metadata.mtime_nsec(),
        metadata.ctime(),
        metadata.ctime_nsec(),
    )
}

#[cfg(test)]
#[path = "evidence_filesystem_tests.rs"]
mod tests;
