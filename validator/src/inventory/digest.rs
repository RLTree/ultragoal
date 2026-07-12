use super::types::InventoryError;
use crate::context::ReadSession;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::Path;
#[cfg(test)]
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};

#[cfg(unix)]
#[derive(Eq, PartialEq)]
struct FileSnapshot {
    device: u64,
    inode: u64,
    mode: u32,
    links: u64,
    size: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

#[cfg(unix)]
fn snapshot(metadata: &fs::Metadata) -> FileSnapshot {
    FileSnapshot {
        device: metadata.dev(),
        inode: metadata.ino(),
        mode: metadata.mode(),
        links: metadata.nlink(),
        size: metadata.size(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanoseconds: metadata.ctime_nsec(),
    }
}

fn concurrent(path: &Path) -> InventoryError {
    io(path, "file identity changed during inventory read")
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn io(path: &Path, error: impl std::fmt::Display) -> InventoryError {
    InventoryError::Io {
        path: path.to_path_buf(),
        message: error.to_string(),
    }
}

pub(crate) const MAX_INVENTORY_FILE_BYTES: u64 = 64 * 1024 * 1024;

#[cfg(test)]
static TEST_PAUSE_BEFORE_OPEN_MS: AtomicU64 = AtomicU64::new(0);
#[cfg(test)]
static TEST_PAUSE_AFTER_FIRST_READ_MS: AtomicU64 = AtomicU64::new(0);

#[cfg(test)]
pub(super) fn set_test_pauses(before_open_ms: u64, after_first_read_ms: u64) {
    TEST_PAUSE_BEFORE_OPEN_MS.store(before_open_ms, Ordering::SeqCst);
    TEST_PAUSE_AFTER_FIRST_READ_MS.store(after_first_read_ms, Ordering::SeqCst);
}

pub(crate) fn file_identity(
    reads: &ReadSession,
    path: &Path,
) -> Result<(String, Option<u32>), InventoryError> {
    file_identity_with_symlink_policy(reads, path, false)
}

pub(crate) fn file_identity_regular(
    reads: &ReadSession,
    path: &Path,
) -> Result<(String, Option<u32>), InventoryError> {
    file_identity_with_symlink_policy(reads, path, true)
}

fn file_identity_with_symlink_policy(
    reads: &ReadSession,
    path: &Path,
    reject_symlink: bool,
) -> Result<(String, Option<u32>), InventoryError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| io(path, error))?;
    let (digest, mode) = if metadata.file_type().is_symlink() {
        if reject_symlink {
            return Err(io(
                path,
                "inventory target must remain a regular non-symlink file",
            ));
        }
        #[cfg(unix)]
        let before = snapshot(&metadata);
        let target = fs::read_link(path).map_err(|error| io(path, error))?;
        #[cfg(unix)]
        let bytes = target.as_os_str().as_bytes().to_vec();
        #[cfg(not(unix))]
        let bytes = target.to_string_lossy().as_bytes().to_vec();
        let after = fs::symlink_metadata(path).map_err(|error| io(path, error))?;
        #[cfg(unix)]
        if before != snapshot(&after) {
            return Err(concurrent(path));
        }
        #[cfg(unix)]
        let mode = Some(after.permissions().mode());
        #[cfg(not(unix))]
        let mode = None;
        let digest = sha256_hex(&bytes);
        reads
            .observe_symlink(path, &digest, &after)
            .map_err(|error| io(path, error))?;
        (digest, mode)
    } else if metadata.is_file() {
        #[cfg(test)]
        std::thread::sleep(std::time::Duration::from_millis(
            TEST_PAUSE_BEFORE_OPEN_MS.swap(0, Ordering::SeqCst),
        ));
        let mut file = reads.open_regular(path).map_err(|error| io(path, error))?;
        let opened_before = file.metadata().map_err(|error| io(path, error))?;
        #[cfg(unix)]
        if (metadata.dev(), metadata.ino()) != (opened_before.dev(), opened_before.ino()) {
            return Err(concurrent(path));
        }
        let mut hasher = Sha256::new();
        let mut buffer = [0_u8; 16 * 1024];
        let mut total = 0_u64;
        loop {
            let read = file.read(&mut buffer).map_err(|error| io(path, error))?;
            if read == 0 {
                break;
            }
            total = total.saturating_add(read as u64);
            if total > MAX_INVENTORY_FILE_BYTES {
                return Err(io(
                    path,
                    format!("inventory file exceeds {} bytes", MAX_INVENTORY_FILE_BYTES),
                ));
            }
            reads.charge(read as u64).map_err(|error| io(path, error))?;
            hasher.update(&buffer[..read]);
            #[cfg(test)]
            if total == read as u64 {
                std::thread::sleep(std::time::Duration::from_millis(
                    TEST_PAUSE_AFTER_FIRST_READ_MS.swap(0, Ordering::SeqCst),
                ));
            }
        }
        let opened_after = file.metadata().map_err(|error| io(path, error))?;
        let path_after = fs::symlink_metadata(path).map_err(|error| io(path, error))?;
        #[cfg(unix)]
        if snapshot(&opened_before) != snapshot(&opened_after)
            || snapshot(&opened_after) != snapshot(&path_after)
        {
            return Err(concurrent(path));
        }
        #[cfg(unix)]
        let mode = Some(opened_after.permissions().mode());
        #[cfg(not(unix))]
        let mode = None;
        let digest = format!("{:x}", hasher.finalize());
        reads
            .observe_regular(path, &digest, total, &opened_after, &file)
            .map_err(|error| io(path, error))?;
        (digest, mode)
    } else {
        return Err(io(path, "inventory target is not a file or symlink"));
    };
    Ok((digest, mode))
}

pub(crate) fn json_digest(value: &serde_json::Value) -> Result<String, InventoryError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| InventoryError::Serialization(error.to_string()))?;
    Ok(sha256_hex(&bytes))
}
