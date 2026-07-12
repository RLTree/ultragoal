#[cfg(unix)]
use super::descriptor::{Directory, EntryKind, read_file};
use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use crate::distribution::package::TreeObject;
use crate::distribution::reader::validate_relative_path;

#[cfg(unix)]
pub(super) fn inspect(
    root: &Directory,
    maximum_entries: usize,
    maximum_bytes: usize,
) -> Result<Vec<TreeObject>, DistributionError> {
    let mut rows = Vec::new();
    let mut total = 0usize;
    visit(
        root,
        "",
        0,
        &mut rows,
        &mut total,
        maximum_entries,
        maximum_bytes,
    )?;
    rows.sort_by(|left, right| left.path().cmp(right.path()));
    Ok(rows)
}

#[cfg(unix)]
fn visit(
    directory: &Directory,
    prefix: &str,
    depth: usize,
    rows: &mut Vec<TreeObject>,
    total: &mut usize,
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
                let child = directory.open_directory(&name)?;
                visit(
                    &child,
                    &relative,
                    depth + 1,
                    rows,
                    total,
                    maximum_entries,
                    maximum_bytes,
                )?;
            }
            EntryKind::Regular => {
                if rows.len() >= maximum_entries || metadata.links != 1 {
                    return Err(error(if rows.len() >= maximum_entries {
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
