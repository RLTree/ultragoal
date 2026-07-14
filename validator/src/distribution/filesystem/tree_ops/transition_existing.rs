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
