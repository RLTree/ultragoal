use super::directory_entries::names;
use super::observation::{open_at, stat_at};
use super::*;

pub(super) struct OutputClaim {
    pub(super) parent: String,
    pub(super) name: String,
    pub(super) identity: OutputDirectoryIdentity,
}

pub(super) fn rollback(
    claims: &[OutputClaim],
    opened: &BTreeMap<String, File>,
) -> Result<(), RoutineError> {
    for claim in claims.iter().rev() {
        let parent = opened
            .get(&claim.parent)
            .ok_or_else(|| error("routine-production-output-rollback-parent-unbound"))?;
        let child = open_at(parent, &claim.name)?;
        if identity(
            &child
                .metadata()
                .map_err(|_| error("routine-production-output-rollback-stat-failed"))?,
        ) != claim.identity
            || stat_at(parent, &claim.name)? != Some(claim.identity)
            || !names(&child)?.is_empty()
        {
            return Err(error("routine-production-output-rollback-custody-changed"));
        }
        let quarantine = format!(".routine-output-cleanup-{:x}", claim.identity.inode);
        rename_exclusive(parent, &claim.name, &quarantine)?;
        let moved = open_at(parent, &quarantine)?;
        if identity(
            &moved
                .metadata()
                .map_err(|_| error("routine-production-output-rollback-stat-failed"))?,
        ) != claim.identity
            || !names(&moved)?.is_empty()
        {
            return Err(error("routine-production-output-rollback-custody-changed"));
        }
        remove_directory(parent, &quarantine)?;
        parent
            .sync_all()
            .map_err(|_| error("routine-production-output-rollback-sync-failed"))?;
    }
    Ok(())
}

fn rename_exclusive(parent: &File, from: &str, to: &str) -> Result<(), RoutineError> {
    let from = validate_name(from)?;
    let to = validate_name(to)?;
    // SAFETY: `parent` is live and both validated names are NUL-terminated
    // components; `RENAME_EXCL` refuses replacement of another directory.
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
        return Err(error(
            "routine-production-output-rollback-quarantine-failed",
        ));
    }
    Ok(())
}

fn remove_directory(parent: &File, name: &str) -> Result<(), RoutineError> {
    let name = validate_name(name)?;
    // SAFETY: `parent` is live and `name` identifies the checked, empty directory.
    if unsafe { libc::unlinkat(parent.as_raw_fd(), name.as_ptr(), libc::AT_REMOVEDIR) } != 0 {
        return Err(error("routine-production-output-rollback-remove-failed"));
    }
    Ok(())
}
