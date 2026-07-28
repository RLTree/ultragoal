use super::*;

use std::ffi::CString;
use std::fs;
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStrExt;

use crate::routine_work::runtime_adapter::mediator::{ObjectIdentity, StagedProgram};

pub(super) struct EntryClaim {
    pub(super) path: PathBuf,
    pub(super) identity: ObjectIdentity,
    pub(super) bytes: Option<Vec<u8>>,
}

pub(super) fn cleanup_partial_stage(
    child: &Path,
    directory_identity: ObjectIdentity,
    claims: &[EntryClaim],
) -> Result<(), RoutineError> {
    let directory =
        fs::File::open(child).map_err(|_| error("routine-production-launch-directory-missing"))?;
    if ObjectIdentity::from(
        &directory
            .metadata()
            .map_err(|_| error("routine-production-launch-directory-stat-failed"))?,
    ) != directory_identity
    {
        return Err(error("routine-production-launch-directory-mismatch"));
    }
    let parent = child
        .parent()
        .ok_or_else(|| error("routine-production-launch-parent-missing"))?;
    let child_name = child
        .file_name()
        .ok_or_else(|| error("routine-production-launch-directory-name-invalid"))?;
    let quarantine_name = format!(
        ".routine-cleanup-{}-{:x}",
        child_name.to_string_lossy(),
        directory_identity.inode
    );
    let quarantine = parent.join(&quarantine_name);
    let parent_file = fs::File::open(parent)
        .map_err(|_| error("routine-production-launch-parent-open-failed"))?;
    rename_exclusive(
        &parent_file,
        child_name.as_bytes(),
        quarantine_name.as_bytes(),
    )?;
    let quarantined = fs::File::open(&quarantine)
        .map_err(|_| error("routine-production-launch-quarantine-open-failed"))?;
    let moved_identity = ObjectIdentity::from(
        &quarantined
            .metadata()
            .map_err(|_| error("routine-production-launch-quarantine-stat-failed"))?,
    );
    if moved_identity.device != directory_identity.device
        || moved_identity.inode != directory_identity.inode
        || moved_identity.mode != directory_identity.mode
        || moved_identity.owner_user_id != directory_identity.owner_user_id
        || moved_identity.owner_group_id != directory_identity.owner_group_id
        || moved_identity.links != directory_identity.links
    {
        return Err(error("routine-production-launch-quarantine-mismatch"));
    }
    let entries = fs::read_dir(&quarantine)
        .map_err(|_| error("routine-production-launch-directory-read-failed"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| error("routine-production-launch-directory-read-failed"))?;
    if entries.len() != claims.len()
        || entries.iter().any(|entry| {
            !claims
                .iter()
                .any(|claim| entry.file_name() == claim.path.file_name().unwrap_or_default())
        })
    {
        return Err(error(
            "routine-production-launch-directory-contents-invalid",
        ));
    }
    for claim in claims {
        let name = claim
            .path
            .file_name()
            .ok_or_else(|| error("routine-production-launch-entry-name-invalid"))?;
        let current_path = quarantine.join(name);
        let current = fs::symlink_metadata(&current_path)
            .map_err(|_| error("routine-production-launch-entry-missing"))?;
        if ObjectIdentity::from(&current) != claim.identity || current.file_type().is_symlink() {
            return Err(error("routine-production-launch-entry-mismatch"));
        }
        if let Some(expected) = &claim.bytes
            && fs::read(&current_path).ok().as_deref() != Some(expected.as_slice())
        {
            return Err(error("routine-production-launch-entry-bytes-mismatch"));
        }
    }
    for claim in claims
        .iter()
        .rev()
        .filter(|claim| claim.path.file_name().and_then(|name| name.to_str()) != Some("authority"))
    {
        fs::remove_file(quarantine.join(claim.path.file_name().unwrap_or_default()))
            .map_err(|_| error("routine-production-launch-entry-cleanup-failed"))?;
    }
    if let Some(marker) = claims
        .iter()
        .find(|claim| claim.path.file_name().and_then(|name| name.to_str()) == Some("authority"))
    {
        fs::remove_file(quarantine.join(marker.path.file_name().unwrap_or_default()))
            .map_err(|_| error("routine-production-launch-entry-cleanup-failed"))?;
    }
    fs::remove_dir(&quarantine)
        .map_err(|_| error("routine-production-launch-directory-cleanup-failed"))?;
    parent_file
        .sync_all()
        .map_err(|_| error("routine-production-launch-parent-sync-failed"))?;
    Ok(())
}

#[cfg(target_vendor = "apple")]
fn rename_exclusive(parent: &fs::File, from: &[u8], to: &[u8]) -> Result<(), RoutineError> {
    let from = CString::new(from)
        .map_err(|_| error("routine-production-launch-directory-name-invalid"))?;
    let to =
        CString::new(to).map_err(|_| error("routine-production-launch-directory-name-invalid"))?;
    // SAFETY: `parent` is a live parent-directory descriptor; both names are
    // validated NUL-terminated entry names and `RENAME_EXCL` prevents replacement.
    if unsafe {
        libc::renameatx_np(
            parent.as_raw_fd(),
            from.as_ptr(),
            parent.as_raw_fd(),
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
fn rename_exclusive(_parent: &fs::File, _from: &[u8], _to: &[u8]) -> Result<(), RoutineError> {
    Err(error("routine-production-launch-host-unsupported"))
}

pub(in crate::routine_work::runtime_adapter::production) fn cleanup_staged(
    staged: &StagedProgram,
) -> Result<(), RoutineError> {
    if ObjectIdentity::from(
        &staged
            .directory_file
            .metadata()
            .map_err(|_| error("routine-production-launch-directory-stat-failed"))?,
    ) != staged.directory_identity
    {
        return Err(error("routine-production-launch-directory-mismatch"));
    }
    cleanup_partial_stage(
        &staged.directory,
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
