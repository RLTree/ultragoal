#[cfg(unix)]
use super::descriptor::{Directory, EntryKind, unlink_directory_identity, unlink_entry_identity};
use crate::distribution::error::{DistributionError, DistributionErrorId, error};

const REMOVE_ENTRY_LIMIT: usize = 4096;
const REMOVE_DEPTH_LIMIT: usize = 64;

#[cfg(unix)]
pub(super) fn remove_tree(parent: &Directory, name: &str) -> Result<(), DistributionError> {
    let Some(metadata) = parent.stat(name)? else {
        return Ok(());
    };
    if metadata.kind != EntryKind::Directory || metadata.identity.device != parent.root_device() {
        return Err(error(DistributionErrorId::UnsafeObject));
    }
    let directory = parent.open_directory(name)?;
    let mut visited = 0usize;
    remove_children(&directory, 0, &mut visited)?;
    let current = parent.stat(name)?;
    if !current
        .is_some_and(|row| row.kind == EntryKind::Directory && row.identity == directory.identity())
    {
        return Err(error(DistributionErrorId::ObjectChanged));
    }
    unlink_directory_identity(parent, name, directory.identity())
}

#[cfg(unix)]
fn remove_children(
    directory: &Directory,
    depth: usize,
    visited: &mut usize,
) -> Result<(), DistributionError> {
    if depth >= REMOVE_DEPTH_LIMIT {
        return Err(error(DistributionErrorId::ObjectTooLarge));
    }
    for name in directory.names()? {
        *visited = visited
            .checked_add(1)
            .ok_or_else(|| error(DistributionErrorId::ObjectTooLarge))?;
        if *visited > REMOVE_ENTRY_LIMIT {
            return Err(error(DistributionErrorId::ObjectTooLarge));
        }
        let metadata = directory
            .stat(&name)?
            .ok_or_else(|| error(DistributionErrorId::ObjectChanged))?;
        if metadata.identity.device != directory.root_device() {
            return Err(error(DistributionErrorId::UnsafeObject));
        }
        if metadata.kind == EntryKind::Directory {
            let child = directory.open_directory(&name)?;
            remove_children(&child, depth + 1, visited)?;
            let current = directory.stat(&name)?;
            if !current.is_some_and(|row| row.identity == child.identity()) {
                return Err(error(DistributionErrorId::ObjectChanged));
            }
            unlink_directory_identity(directory, &name, child.identity())?;
        } else {
            if metadata.kind != EntryKind::Regular || metadata.links != 1 {
                return Err(error(DistributionErrorId::UnsafeObject));
            }
            unlink_entry_identity(directory, &name, metadata)?;
        }
    }
    Ok(())
}
