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
pub(super) struct TreeTransition<'a> {
    pub(super) root: &'a Directory,
    pub(super) target_parent: &'a Directory,
    pub(super) target: &'a str,
    pub(super) stage_name: &'a str,
    pub(super) backup_name: &'a str,
    pub(super) before: Option<&'a TreeSnapshot>,
    pub(super) replacement: Option<&'a [TreeObject]>,
    pub(super) stage: Option<&'a mut OwnedTree>,
}

#[cfg(unix)]
pub(super) fn transition_tree(request: TreeTransition<'_>) -> Result<bool, DistributionError> {
    let TreeTransition {
        root,
        target_parent,
        target,
        stage_name,
        backup_name,
        before,
        replacement,
        stage,
    } = request;
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
        (Some(expected), replacement, stage) => transition_existing(ExistingTreeTransition {
            root,
            target_parent,
            target,
            stage_name,
            backup_name,
            expected,
            replacement,
            stage,
        }),
        _ => Err(error(DistributionErrorId::EffectFailed)),
    }
}
