use super::root::ConfinedRoot;
use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use crate::distribution::package::{MaterializeEffects, TreeObject, tree_sha256};
use crate::distribution::reader::sha256;
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(unix)]
use super::descriptor::{Directory, EntryKind, rename_noreplace};
#[cfg(unix)]
use super::owned::OwnedFile;
#[cfg(unix)]
use super::remove::remove_tree;
#[cfg(unix)]
use super::tree_ops::{OwnedTree, debris, same_snapshot, snapshot_at, transition_tree, write_tree};

static NONCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug)]
pub struct ScopedTree {
    root: ConfinedRoot,
    relative: String,
}

impl ScopedTree {
    pub fn new(root: ConfinedRoot, relative: &str) -> Result<Self, DistributionError> {
        root.validate_relative(relative)?;
        Ok(Self {
            root,
            relative: relative.into(),
        })
    }

    pub(crate) fn root_id(&self) -> &str {
        self.root.root_id()
    }

    pub(crate) fn relative_path(&self) -> &str {
        &self.relative
    }

    #[cfg(unix)]
    pub fn inspect(
        &self,
        maximum_entries: usize,
        maximum_bytes: usize,
    ) -> Result<Option<Vec<TreeObject>>, DistributionError> {
        let (parent, name) = match self.root.parent(&self.relative, false) {
            Ok(value) => value,
            Err(failure) if failure.id() == DistributionErrorId::ObjectUnavailable => {
                return Ok(None);
            }
            Err(failure) => return Err(failure),
        };
        let snapshot = snapshot_at(&parent, &name, maximum_entries, maximum_bytes)?;
        self.root.revalidate_parent(&self.relative, &parent)?;
        Ok(snapshot.map(|row| row.rows))
    }

    #[cfg(not(unix))]
    pub fn inspect(
        &self,
        _maximum_entries: usize,
        _maximum_bytes: usize,
    ) -> Result<Option<Vec<TreeObject>>, DistributionError> {
        Err(error(DistributionErrorId::CapabilityMismatch))
    }

    #[cfg(unix)]
    fn apply(
        &self,
        expected: Option<&str>,
        replacement: Option<&[TreeObject]>,
    ) -> Result<bool, DistributionError> {
        let root = self.root.root_directory()?;
        let token_digest = sha256(self.relative.as_bytes());
        let token = &token_digest[7..];
        let lock_name = format!(".hul-tree-{token}-lock");
        let _lock = OwnedFile::create(root.duplicate()?, lock_name, &[], 0o600, true)?;
        if !debris(&root, token)?.is_empty() {
            return Err(error(DistributionErrorId::InstallConflict));
        }
        let (parent, name) = self.root.parent(&self.relative, true)?;
        let before = snapshot_at(&parent, &name, 4096, 64 * 1024 * 1024)?;
        if before
            .as_ref()
            .map(|row| tree_sha256(&row.rows))
            .transpose()?
            .as_deref()
            != expected
        {
            return Ok(false);
        }
        let nonce = NONCE.fetch_add(1, Ordering::Relaxed);
        let stage_name = format!(".hul-tree-{token}-stage-{}-{nonce}", std::process::id());
        let backup_name = format!(".hul-tree-{token}-backup-{}-{nonce}", std::process::id());
        let mut stage = replacement
            .map(|rows| {
                let stage = OwnedTree::create(root.duplicate()?, stage_name.clone())?;
                write_tree(&stage.open()?, rows)?;
                let staged = snapshot_at(&root, &stage_name, 4096, 64 * 1024 * 1024)?
                    .ok_or_else(|| error(DistributionErrorId::EffectFailed))?;
                if staged.rows != rows {
                    return Err(error(DistributionErrorId::ArchiveMismatch));
                }
                Ok(stage)
            })
            .transpose()?;
        let current = snapshot_at(&parent, &name, 4096, 64 * 1024 * 1024)?;
        if !same_snapshot(current.as_ref(), before.as_ref()) {
            return Ok(false);
        }
        self.root.revalidate_parent(&self.relative, &parent)?;
        let transitioned = transition_tree(
            &root,
            &parent,
            &name,
            &stage_name,
            &backup_name,
            before.as_ref(),
            replacement,
            stage.as_mut(),
        )?;
        if !transitioned {
            return Ok(false);
        }
        self.root.revalidate_parent(&self.relative, &parent)?;
        let after = snapshot_at(&parent, &name, 4096, 64 * 1024 * 1024)?;
        if after.as_ref().map(|row| row.rows.as_slice()) != replacement {
            return Err(error(DistributionErrorId::ObjectChanged));
        }
        Ok(true)
    }

    #[cfg(not(unix))]
    fn apply(
        &self,
        _expected: Option<&str>,
        _replacement: Option<&[TreeObject]>,
    ) -> Result<bool, DistributionError> {
        Err(error(DistributionErrorId::CapabilityMismatch))
    }

    #[cfg(unix)]
    pub fn recover_interrupted(&self) -> Result<bool, DistributionError> {
        let root = self.root.root_directory()?;
        let token_digest = sha256(self.relative.as_bytes());
        let token = &token_digest[7..];
        let lock_name = format!(".hul-tree-{token}-lock");
        let _lock = OwnedFile::create(root.duplicate()?, lock_name, &[], 0o600, true)?;
        let (parent, name) = self.root.parent(&self.relative, true)?;
        let debris = debris(&root, token)?;
        let backups = debris.iter().filter(|(_, backup)| *backup).count();
        let target_present = target_directory_present(&parent, &name)?;
        if backups > 1 || backups == 1 && target_present {
            return Err(error(DistributionErrorId::InstallConflict));
        }
        let mut changed = false;
        for (debris_name, backup) in debris {
            if backup {
                restore_recovery_backup(&root, &debris_name, &parent, &name)?;
            } else {
                remove_tree(&root, &debris_name)?;
            }
            changed = true;
        }
        self.root.revalidate_parent(&self.relative, &parent)?;
        Ok(changed)
    }

    #[cfg(not(unix))]
    pub fn recover_interrupted(&self) -> Result<bool, DistributionError> {
        Err(error(DistributionErrorId::CapabilityMismatch))
    }
}

#[cfg(unix)]
fn target_directory_present(parent: &Directory, name: &str) -> Result<bool, DistributionError> {
    match parent.stat(name)? {
        None => Ok(false),
        Some(row) if row.kind == EntryKind::Directory => Ok(true),
        Some(_) => Err(error(DistributionErrorId::UnsafeObject)),
    }
}

#[cfg(unix)]
fn restore_recovery_backup(
    root: &Directory,
    backup: &str,
    target_parent: &Directory,
    target: &str,
) -> Result<(), DistributionError> {
    let expected = snapshot_at(root, backup, 4096, 64 * 1024 * 1024)?
        .ok_or_else(|| error(DistributionErrorId::ObjectChanged))?;
    if !rename_noreplace(root, backup, target_parent, target)? {
        return Err(error(if target_parent.stat(target)?.is_some() {
            DistributionErrorId::InstallConflict
        } else {
            DistributionErrorId::ObjectChanged
        }));
    }
    let current = snapshot_at(target_parent, target, 4096, 64 * 1024 * 1024);
    match current {
        Ok(Some(current)) if same_snapshot(Some(&current), Some(&expected)) => Ok(()),
        current => {
            match rename_noreplace(target_parent, target, root, backup) {
                Ok(true) => {}
                _ => return Err(error(DistributionErrorId::RollbackFailed)),
            }
            match current {
                Err(failure) => Err(failure),
                _ => Err(error(DistributionErrorId::ObjectChanged)),
            }
        }
    }
}

impl MaterializeEffects for ScopedTree {
    fn read_tree(&mut self, entries: usize, bytes: usize) -> Result<Option<Vec<TreeObject>>, ()> {
        self.inspect(entries, bytes).map_err(|_| ())
    }

    fn compare_exchange_tree(
        &mut self,
        expected_sha256: Option<&str>,
        replacement: Option<&[TreeObject]>,
    ) -> Result<bool, ()> {
        self.apply(expected_sha256, replacement).map_err(|_| ())
    }
}
