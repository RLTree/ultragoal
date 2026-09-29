use super::*;

use std::ffi::{CStr, CString};
use std::fs::File;
use std::io::Read;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;

use crate::routine_work::runtime_adapter::mediator::{ObjectIdentity, StagedProgram};

use super::snapshot::validate_launch_root;

pub(super) struct EntryClaim {
    pub(super) path: PathBuf,
    pub(super) identity: ObjectIdentity,
    pub(super) bytes: Option<Vec<u8>>,
}

pub(super) fn cleanup_partial_stage(
    root: &Path,
    directory: &File,
    directory_identity: ObjectIdentity,
    claims: &[EntryClaim],
) -> Result<(), RoutineError> {
    validate_launch_root(root, directory, directory_identity)?;
    for claim in claims.iter().rev() {
        quarantine_and_remove(directory, claim)?;
        validate_launch_root(root, directory, directory_identity)?;
    }
    directory
        .sync_all()
        .map_err(|_| error("routine-production-launch-directory-sync-failed"))?;
    validate_launch_root(root, directory, directory_identity)
}

fn quarantine_and_remove(directory: &File, claim: &EntryClaim) -> Result<(), RoutineError> {
    let original = component_name(&claim.path)?;
    let quarantine = CString::new(format!(
        ".routine-cleanup-{}-{:x}",
        original.to_string_lossy(),
        claim.identity.inode
    ))
    .map_err(|_| error("routine-production-launch-entry-name-invalid"))?;
    let original_open = open_optional(directory, &original)?;
    let quarantine_open = open_optional(directory, &quarantine)?;
    match (original_open, quarantine_open) {
        (Some(_), Some(_)) => {
            return Err(error("routine-production-launch-quarantine-conflict"));
        }
        (Some(_), None) => rename_exclusive(directory, &original, &quarantine)?,
        (None, Some(_)) => {}
        (None, None) => return Err(error("routine-production-launch-entry-missing")),
    }
    let mut quarantined = open_optional(directory, &quarantine)?
        .ok_or_else(|| error("routine-production-launch-quarantine-missing"))?;
    if !same_leaf_after_rename(
        claim.identity,
        ObjectIdentity::from(
            &quarantined
                .metadata()
                .map_err(|_| error("routine-production-launch-quarantine-stat-failed"))?,
        ),
    ) {
        return Err(error("routine-production-launch-quarantine-mismatch"));
    }
    if let Some(expected) = &claim.bytes {
        let mut observed = Vec::with_capacity(expected.len().saturating_add(1));
        quarantined
            .by_ref()
            .take(expected.len().saturating_add(1) as u64)
            .read_to_end(&mut observed)
            .map_err(|_| error("routine-production-launch-entry-read-failed"))?;
        if observed != *expected {
            return Err(error("routine-production-launch-entry-bytes-mismatch"));
        }
    }
    unlink_entry(directory, &quarantine)?;
    if open_optional(directory, &quarantine)?.is_some() {
        return Err(error("routine-production-launch-entry-cleanup-failed"));
    }
    Ok(())
}

fn same_leaf_after_rename(expected: ObjectIdentity, current: ObjectIdentity) -> bool {
    expected.device == current.device
        && expected.inode == current.inode
        && expected.mode == current.mode
        && expected.owner_user_id == current.owner_user_id
        && expected.owner_group_id == current.owner_group_id
        && expected.links == current.links
        && expected.length == current.length
        && expected.modified_seconds == current.modified_seconds
        && expected.modified_nanos == current.modified_nanos
}

fn component_name(path: &Path) -> Result<CString, RoutineError> {
    let name = path
        .file_name()
        .ok_or_else(|| error("routine-production-launch-entry-name-invalid"))?;
    CString::new(name.as_bytes()).map_err(|_| error("routine-production-launch-entry-name-invalid"))
}

fn open_optional(directory: &File, name: &CStr) -> Result<Option<File>, RoutineError> {
    // SAFETY: `directory` is a held directory descriptor, `name` is one
    // NUL-terminated component, and success returns one newly owned descriptor.
    let descriptor = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        )
    };
    if descriptor >= 0 {
        // SAFETY: the successful descriptor is newly owned.
        return Ok(Some(unsafe { File::from_raw_fd(descriptor) }));
    }
    let failure = std::io::Error::last_os_error();
    if failure.raw_os_error() == Some(libc::ENOENT) {
        Ok(None)
    } else {
        Err(error("routine-production-launch-entry-open-failed"))
    }
}

#[cfg(target_vendor = "apple")]
fn rename_exclusive(directory: &File, from: &CStr, to: &CStr) -> Result<(), RoutineError> {
    // SAFETY: `directory` is a held directory descriptor, both names are
    // NUL-terminated single components, and `RENAME_EXCL` forbids replacement.
    if unsafe {
        libc::renameatx_np(
            directory.as_raw_fd(),
            from.as_ptr(),
            directory.as_raw_fd(),
            to.as_ptr(),
            libc::RENAME_EXCL,
        )
    } != 0
    {
        return Err(error("routine-production-launch-quarantine-failed"));
    }
    Ok(())
}

#[cfg(not(target_vendor = "apple"))]
fn rename_exclusive(_directory: &File, _from: &CStr, _to: &CStr) -> Result<(), RoutineError> {
    Err(error("routine-production-launch-host-unsupported"))
}

fn unlink_entry(directory: &File, name: &CStr) -> Result<(), RoutineError> {
    // SAFETY: `directory` is a held directory descriptor and `name` is one
    // NUL-terminated component. No path traversal or recursive deletion occurs.
    if unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), 0) } != 0 {
        return Err(error("routine-production-launch-entry-cleanup-failed"));
    }
    Ok(())
}

pub(in crate::routine_work::runtime_adapter::production) fn cleanup_staged(
    staged: &StagedProgram,
) -> Result<(), RoutineError> {
    cleanup_partial_stage(
        &staged.directory,
        &staged.directory_file,
        staged.directory_identity,
        &[
            EntryClaim {
                path: staged.executable.path().to_path_buf(),
                identity: staged.executable.identity,
                bytes: None,
            },
            EntryClaim {
                path: staged.seal.clone(),
                identity: staged.seal_identity,
                bytes: Some(staged.seal_bytes.clone()),
            },
            EntryClaim {
                path: staged.marker.clone(),
                identity: staged.marker_identity,
                bytes: Some(staged.marker_bytes.clone()),
            },
        ],
    )
}

#[cfg(all(test, target_vendor = "apple"))]
mod tests {
    use super::*;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct Fixture {
        root: PathBuf,
        directory: File,
        identity: ObjectIdentity,
    }

    impl Fixture {
        fn new(label: &str) -> Self {
            let suffix = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos();
            let root = std::env::temp_dir().join(format!("launch-cleanup-{label}-{suffix}"));
            std::fs::create_dir(&root).expect("launch root");
            let directory = File::open(&root).expect("hold launch root");
            let identity = ObjectIdentity::from(&directory.metadata().expect("root metadata"));
            Self {
                root,
                directory,
                identity,
            }
        }

        fn claim(&self, name: &str, bytes: &[u8]) -> EntryClaim {
            let path = self.root.join(name);
            let mut file = File::create(&path).expect("create claim");
            file.write_all(bytes).expect("write claim");
            file.sync_all().expect("sync claim");
            EntryClaim {
                path,
                identity: ObjectIdentity::from(&file.metadata().expect("claim metadata")),
                bytes: Some(bytes.to_vec()),
            }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn exact_cleanup_preserves_unrelated_entries() {
        let fixture = Fixture::new("unrelated");
        let claim = fixture.claim("launch-bound-program", b"owned\n");
        std::fs::write(fixture.root.join("unrelated"), b"keep\n").expect("unrelated");

        cleanup_partial_stage(
            &fixture.root,
            &fixture.directory,
            fixture.identity,
            &[claim],
        )
        .expect("exact cleanup");

        assert!(!fixture.root.join("launch-bound-program").exists());
        assert_eq!(
            std::fs::read(fixture.root.join("unrelated")).unwrap(),
            b"keep\n"
        );
    }

    #[test]
    fn substituted_leaf_is_quarantined_and_preserved_without_deletion() {
        let fixture = Fixture::new("substitute");
        let claim = fixture.claim("launch-bound-program", b"owned\n");
        let retained_owned = fixture.root.join("retained-owned");
        std::fs::rename(&claim.path, &retained_owned).expect("move owned leaf");
        std::fs::write(&claim.path, b"substitute\n").expect("substitute leaf");

        let failure = cleanup_partial_stage(
            &fixture.root,
            &fixture.directory,
            fixture.identity,
            &[claim],
        )
        .expect_err("substitution must fail closed");

        assert_eq!(
            failure.cause(),
            "routine-production-launch-quarantine-mismatch"
        );
        assert_eq!(std::fs::read(retained_owned).unwrap(), b"owned\n");
        let quarantined = std::fs::read_dir(&fixture.root)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .find(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with(".routine-cleanup-"))
            })
            .expect("substitute retained in quarantine");
        assert_eq!(std::fs::read(quarantined).unwrap(), b"substitute\n");
    }

    #[test]
    fn cleanup_recovers_original_absent_quarantine_present() {
        let fixture = Fixture::new("recovery");
        let claim = fixture.claim("launch-bound-program", b"owned\n");
        let quarantine = fixture.root.join(format!(
            ".routine-cleanup-launch-bound-program-{:x}",
            claim.identity.inode
        ));
        std::fs::rename(&claim.path, &quarantine).expect("simulate completed quarantine rename");

        cleanup_partial_stage(
            &fixture.root,
            &fixture.directory,
            fixture.identity,
            &[claim],
        )
        .expect("quarantine recovery");

        assert!(!quarantine.exists());
    }
}
