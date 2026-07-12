#[cfg(unix)]
use super::descriptor::{Directory, DirectoryIdentity, EntryKind, create_file, rename_noreplace};
#[cfg(unix)]
use super::remove::remove_tree;
use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use crate::distribution::package::TreeObject;
use crate::distribution::reader::validate_relative_path;

#[cfg(unix)]
#[derive(Debug)]
pub(super) struct TreeSnapshot {
    pub(super) identity: DirectoryIdentity,
    pub(super) rows: Vec<TreeObject>,
}

#[cfg(unix)]
pub(super) fn snapshot_at(
    parent: &Directory,
    name: &str,
    entries: usize,
    bytes: usize,
) -> Result<Option<TreeSnapshot>, DistributionError> {
    let Some(metadata) = parent.stat(name)? else {
        return Ok(None);
    };
    if metadata.kind != EntryKind::Directory || metadata.identity.device != parent.root_device() {
        return Err(error(DistributionErrorId::UnsafeObject));
    }
    let directory = parent.open_directory(name)?;
    let rows = super::walk::inspect(&directory, entries, bytes)?;
    if !parent
        .stat(name)?
        .is_some_and(|row| row.identity == directory.identity())
    {
        return Err(error(DistributionErrorId::ObjectChanged));
    }
    Ok(Some(TreeSnapshot {
        identity: directory.identity(),
        rows,
    }))
}

#[cfg(unix)]
pub(super) fn same_snapshot(left: Option<&TreeSnapshot>, right: Option<&TreeSnapshot>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => left.identity == right.identity && left.rows == right.rows,
        _ => false,
    }
}

#[cfg(unix)]
pub(super) struct OwnedTree {
    parent: Directory,
    name: String,
    identity: DirectoryIdentity,
    active: bool,
}

#[cfg(unix)]
impl OwnedTree {
    pub(super) fn create(parent: Directory, name: String) -> Result<Self, DistributionError> {
        let directory = parent.create_directory(&name)?;
        Ok(Self {
            parent,
            name,
            identity: directory.identity(),
            active: true,
        })
    }

    pub(super) fn open(&self) -> Result<Directory, DistributionError> {
        self.require_current()?;
        self.parent.open_directory(&self.name)
    }

    pub(super) fn require_current(&self) -> Result<(), DistributionError> {
        self.require_at(&self.parent, &self.name)
    }

    pub(super) fn require_at(
        &self,
        parent: &Directory,
        name: &str,
    ) -> Result<(), DistributionError> {
        if !parent
            .stat(name)?
            .is_some_and(|row| row.kind == EntryKind::Directory && row.identity == self.identity)
        {
            return Err(error(DistributionErrorId::ObjectChanged));
        }
        Ok(())
    }

    pub(super) fn identity(&self) -> DirectoryIdentity {
        self.identity
    }

    pub(super) fn disarm(&mut self) {
        self.active = false;
    }
}

#[cfg(unix)]
impl Drop for OwnedTree {
    fn drop(&mut self) {
        if self.active && self.require_current().is_ok() {
            let _ = remove_tree(&self.parent, &self.name);
        }
    }
}

#[cfg(unix)]
pub(super) fn write_tree(root: &Directory, rows: &[TreeObject]) -> Result<(), DistributionError> {
    for row in rows {
        validate_relative_path(row.path())?;
        let components = row.path().split('/').collect::<Vec<_>>();
        let mut parent = root.duplicate()?;
        for component in &components[..components.len() - 1] {
            parent = parent.ensure_directory(component)?;
        }
        let leaf = components[components.len() - 1];
        create_file(&parent, leaf, row.bytes(), row.mode())?;
    }
    Ok(())
}

#[cfg(unix)]
pub(super) fn transition_tree(
    root: &Directory,
    target_parent: &Directory,
    target: &str,
    stage_name: &str,
    backup_name: &str,
    before: Option<&TreeSnapshot>,
    replacement: Option<&[TreeObject]>,
    stage: Option<&mut OwnedTree>,
) -> Result<bool, DistributionError> {
    match (before, replacement, stage) {
        (None, None, None) => Ok(true),
        (None, Some(replacement), Some(stage)) => {
            stage.require_current()?;
            if !rename_noreplace(root, stage_name, target_parent, target)? {
                return Ok(false);
            }
            match replacement_tree_current(stage, target_parent, target, replacement) {
                Ok(true) => {
                    stage.disarm();
                    Ok(true)
                }
                Ok(false) => {
                    stage.disarm();
                    rollback_rename(target_parent, target, root, stage_name)?;
                    Ok(false)
                }
                Err(failure) => {
                    stage.disarm();
                    rollback_rename(target_parent, target, root, stage_name)?;
                    Err(failure)
                }
            }
        }
        (Some(expected), replacement, stage) => transition_existing(
            root,
            target_parent,
            target,
            stage_name,
            backup_name,
            expected,
            replacement,
            stage,
        ),
        _ => Err(error(DistributionErrorId::EffectFailed)),
    }
}

#[cfg(unix)]
fn transition_existing(
    root: &Directory,
    target_parent: &Directory,
    target: &str,
    stage_name: &str,
    backup_name: &str,
    expected: &TreeSnapshot,
    replacement: Option<&[TreeObject]>,
    mut stage: Option<&mut OwnedTree>,
) -> Result<bool, DistributionError> {
    if replacement.is_some() != stage.is_some() {
        return Err(error(DistributionErrorId::EffectFailed));
    }
    if !rename_noreplace(target_parent, target, root, backup_name)? {
        return Ok(false);
    }
    let captured = snapshot_at(root, backup_name, 4096, 64 * 1024 * 1024)?;
    if !same_snapshot(captured.as_ref(), Some(expected)) {
        restore_backup(root, backup_name, target_parent, target)?;
        return Ok(false);
    }
    if let Some(stage) = stage.as_mut() {
        let replacement = replacement.ok_or_else(|| error(DistributionErrorId::EffectFailed))?;
        if let Err(failure) = stage.require_current() {
            restore_backup(root, backup_name, target_parent, target)?;
            return Err(failure);
        }
        if !rename_noreplace(root, stage_name, target_parent, target)? {
            restore_backup(root, backup_name, target_parent, target)?;
            return Err(error(DistributionErrorId::InstallConflict));
        }
        match replacement_tree_current(stage, target_parent, target, replacement) {
            Ok(true) => {}
            Ok(false) => {
                stage.disarm();
                rollback_rename(target_parent, target, root, stage_name)?;
                restore_backup(root, backup_name, target_parent, target)?;
                return Ok(false);
            }
            Err(failure) => {
                stage.disarm();
                rollback_rename(target_parent, target, root, stage_name)?;
                restore_backup(root, backup_name, target_parent, target)?;
                return Err(failure);
            }
        }
        stage.disarm();
    }
    remove_tree(root, backup_name)?;
    Ok(true)
}

#[cfg(unix)]
fn replacement_tree_current(
    stage: &OwnedTree,
    parent: &Directory,
    name: &str,
    replacement: &[TreeObject],
) -> Result<bool, DistributionError> {
    stage.require_at(parent, name)?;
    Ok(
        snapshot_at(parent, name, 4096, 64 * 1024 * 1024)?.is_some_and(|snapshot| {
            snapshot.identity == stage.identity() && snapshot.rows == replacement
        }),
    )
}

#[cfg(unix)]
fn rollback_rename(
    from_parent: &Directory,
    from: &str,
    to_parent: &Directory,
    to: &str,
) -> Result<(), DistributionError> {
    match rename_noreplace(from_parent, from, to_parent, to) {
        Ok(true) => Ok(()),
        _ => Err(error(DistributionErrorId::RollbackFailed)),
    }
}

#[cfg(unix)]
fn restore_backup(
    root: &Directory,
    backup: &str,
    target_parent: &Directory,
    target: &str,
) -> Result<(), DistributionError> {
    if !rename_noreplace(root, backup, target_parent, target)? {
        return Err(error(DistributionErrorId::RollbackFailed));
    }
    Ok(())
}

#[cfg(unix)]
pub(super) fn debris(
    root: &Directory,
    token: &str,
) -> Result<Vec<(String, bool)>, DistributionError> {
    let stage = format!(".hul-tree-{token}-stage-");
    let backup = format!(".hul-tree-{token}-backup-");
    Ok(root
        .names()?
        .into_iter()
        .filter_map(|name| {
            if name.starts_with(&stage) {
                Some((name, false))
            } else if name.starts_with(&backup) {
                Some((name, true))
            } else {
                None
            }
        })
        .collect())
}
