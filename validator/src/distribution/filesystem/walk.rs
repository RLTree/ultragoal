#[cfg(unix)]
use super::descriptor::{Directory, DirectoryObservationMetadata, EntryKind, read_file};
use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use crate::distribution::package::TreeObject;
use crate::distribution::reader::validate_relative_path;
#[cfg(unix)]
use serde::Serialize;

const MAXIMUM_DIRECTORY_COUNT: usize = 4096;

#[cfg(unix)]
pub(super) struct TreeInspection {
    pub(super) files: Vec<TreeObject>,
    pub(super) directories: Vec<DirectoryObservation>,
}

#[cfg(unix)]
#[derive(Debug, Eq, PartialEq, Serialize)]
pub(super) struct DirectoryObservation {
    path: String,
    mode: u32,
    owner: u32,
    group: u32,
}

#[cfg(unix)]
impl DirectoryObservation {
    fn capture(path: String, metadata: DirectoryObservationMetadata) -> Self {
        Self {
            path,
            mode: metadata.mode & 0o7777,
            owner: metadata.owner,
            group: metadata.group,
        }
    }
}

#[cfg(unix)]
pub(super) fn inspect(
    root: &Directory,
    maximum_entries: usize,
    maximum_bytes: usize,
) -> Result<Vec<TreeObject>, DistributionError> {
    inspect_with_directories(root, maximum_entries, maximum_bytes)
        .map(|inspection| inspection.files)
}

#[cfg(unix)]
pub(super) fn inspect_with_directories(
    root: &Directory,
    maximum_entries: usize,
    maximum_bytes: usize,
) -> Result<TreeInspection, DistributionError> {
    let mut rows = Vec::new();
    let root_metadata = root.observation_metadata()?;
    let mut directory_rows = vec![DirectoryObservation::capture(String::new(), root_metadata)];
    let mut total = 0usize;
    let mut directories = 0usize;
    visit(
        root,
        "",
        0,
        &mut rows,
        &mut directory_rows,
        &mut total,
        &mut directories,
        maximum_entries,
        maximum_bytes,
    )?;
    if root.observation_metadata()? != root_metadata {
        return Err(error(DistributionErrorId::ObjectChanged));
    }
    rows.sort_by(|left, right| left.path().cmp(right.path()));
    directory_rows.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(TreeInspection {
        files: rows,
        directories: directory_rows,
    })
}

#[cfg(unix)]
fn visit(
    directory: &Directory,
    prefix: &str,
    depth: usize,
    rows: &mut Vec<TreeObject>,
    directory_rows: &mut Vec<DirectoryObservation>,
    total: &mut usize,
    directories: &mut usize,
    maximum_entries: usize,
    maximum_bytes: usize,
) -> Result<(), DistributionError> {
    if depth > 64 {
        return Err(error(DistributionErrorId::ObjectTooLarge));
    }
    for name in directory.names()? {
        let relative = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        validate_relative_path(&relative)?;
        let metadata = directory
            .stat(&name)?
            .ok_or_else(|| error(DistributionErrorId::ObjectChanged))?;
        if metadata.identity.device != directory.root_device() {
            return Err(error(DistributionErrorId::UnsafeObject));
        }
        match metadata.kind {
            EntryKind::Directory => {
                if *directories == MAXIMUM_DIRECTORY_COUNT {
                    return Err(error(DistributionErrorId::ObjectTooLarge));
                }
                *directories = directories
                    .checked_add(1)
                    .ok_or_else(|| error(DistributionErrorId::ObjectTooLarge))?;
                let child = directory.open_directory(&name)?;
                if child.identity() != metadata.identity {
                    return Err(error(DistributionErrorId::ObjectChanged));
                }
                let child_metadata = child.observation_metadata()?;
                if child_metadata.identity != metadata.identity {
                    return Err(error(DistributionErrorId::ObjectChanged));
                }
                directory_rows.push(DirectoryObservation::capture(
                    relative.clone(),
                    child_metadata,
                ));
                visit(
                    &child,
                    &relative,
                    depth + 1,
                    rows,
                    directory_rows,
                    total,
                    directories,
                    maximum_entries,
                    maximum_bytes,
                )?;
                if child.observation_metadata()? != child_metadata
                    || !directory.stat(&name)?.is_some_and(|current| {
                        current.kind == EntryKind::Directory && current.identity == child.identity()
                    })
                {
                    return Err(error(DistributionErrorId::ObjectChanged));
                }
            }
            EntryKind::Regular => {
                if rows.len() == maximum_entries || metadata.links != 1 {
                    return Err(error(if rows.len() == maximum_entries {
                        DistributionErrorId::ObjectTooLarge
                    } else {
                        DistributionErrorId::UnsafeObject
                    }));
                }
                let remaining = maximum_bytes.saturating_sub(*total);
                let snapshot = read_file(directory, &name, remaining)?
                    .ok_or_else(|| error(DistributionErrorId::ObjectChanged))?;
                *total = total
                    .checked_add(snapshot.bytes.len())
                    .ok_or_else(|| error(DistributionErrorId::ObjectTooLarge))?;
                if *total > maximum_bytes {
                    return Err(error(DistributionErrorId::ObjectTooLarge));
                }
                rows.push(TreeObject::regular(relative, snapshot.mode, snapshot.bytes));
            }
            EntryKind::Other => return Err(error(DistributionErrorId::UnsafeObject)),
        }
    }
    Ok(())
}

#[cfg(not(unix))]
pub(super) fn inspect(
    _root: &(),
    _maximum_entries: usize,
    _maximum_bytes: usize,
) -> Result<Vec<TreeObject>, DistributionError> {
    Err(error(DistributionErrorId::CapabilityMismatch))
}
