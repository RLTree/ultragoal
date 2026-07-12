use super::descriptor::{
    Directory, FileSnapshot, read_file, rename_noreplace, rename_swap, unlink_file_identity,
};
use super::file::FILE_LIMIT;
use super::owned::OwnedFile;
use crate::distribution::error::{DistributionError, DistributionErrorId, error};

pub(super) fn transition(
    root: &Directory,
    target_parent: &Directory,
    target: &str,
    stage_name: &str,
    before: Option<&FileSnapshot>,
    replacement: Option<&[u8]>,
    stage: Option<&mut OwnedFile>,
) -> Result<bool, DistributionError> {
    match (before, replacement, stage) {
        (None, None, None) => Ok(true),
        (None, Some(replacement), Some(stage)) => {
            create_transition(root, target_parent, target, stage_name, replacement, stage)
        }
        (Some(expected), Some(replacement), Some(stage)) => replace_transition(
            root,
            target_parent,
            target,
            stage_name,
            expected,
            replacement,
            stage,
        ),
        (Some(expected), None, None) => {
            remove_transition(root, target_parent, target, stage_name, expected)
        }
        _ => Err(error(DistributionErrorId::EffectFailed)),
    }
}

fn create_transition(
    root: &Directory,
    target_parent: &Directory,
    target: &str,
    stage_name: &str,
    replacement: &[u8],
    stage: &mut OwnedFile,
) -> Result<bool, DistributionError> {
    stage.require_current()?;
    if !rename_noreplace(root, stage_name, target_parent, target)? {
        return Ok(false);
    }
    match replacement_file_current(stage, target_parent, target, replacement) {
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

fn replace_transition(
    root: &Directory,
    target_parent: &Directory,
    target: &str,
    stage_name: &str,
    expected: &FileSnapshot,
    replacement: &[u8],
    stage: &mut OwnedFile,
) -> Result<bool, DistributionError> {
    stage.require_current()?;
    if !rename_swap(root, stage_name, target_parent, target)? {
        return Ok(false);
    }
    let captured = capture_after_swap(root, stage_name, target_parent, target)?;
    if !same_moved_snapshot(&captured, expected) {
        rollback_swap(root, stage_name, target_parent, target)?;
        return Ok(false);
    }
    match replacement_file_current(stage, target_parent, target, replacement) {
        Ok(true) => {}
        Ok(false) => {
            stage.disarm();
            rollback_swap(root, stage_name, target_parent, target)?;
            return Ok(false);
        }
        Err(failure) => {
            stage.disarm();
            rollback_swap(root, stage_name, target_parent, target)?;
            return Err(failure);
        }
    }
    match unlink_file_identity(root, stage_name, captured.identity) {
        Ok(()) => {
            stage.disarm();
            Ok(true)
        }
        Err(failure) => {
            let old_is_restorable = read_file(root, stage_name, FILE_LIMIT)
                .ok()
                .flatten()
                .is_some_and(|current| same_moved_snapshot(&current, &captured));
            let candidate_is_current =
                replacement_file_current(stage, target_parent, target, replacement)
                    .unwrap_or(false);
            if old_is_restorable && candidate_is_current {
                rollback_swap(root, stage_name, target_parent, target)?;
            }
            Err(failure)
        }
    }
}

fn remove_transition(
    root: &Directory,
    target_parent: &Directory,
    target: &str,
    stage_name: &str,
    expected: &FileSnapshot,
) -> Result<bool, DistributionError> {
    if !rename_noreplace(target_parent, target, root, stage_name)? {
        return Ok(false);
    }
    let captured = capture_after_rename(root, stage_name, target_parent, target)?;
    if !same_moved_snapshot(&captured, expected) {
        rollback_rename(root, stage_name, target_parent, target)?;
        return Ok(false);
    }
    match unlink_file_identity(root, stage_name, captured.identity) {
        Ok(()) => Ok(true),
        Err(failure) => {
            if read_file(root, stage_name, FILE_LIMIT)
                .ok()
                .flatten()
                .is_some_and(|current| same_moved_snapshot(&current, &captured))
            {
                rollback_rename(root, stage_name, target_parent, target)?;
            }
            Err(failure)
        }
    }
}

fn capture_after_swap(
    root: &Directory,
    stage_name: &str,
    target_parent: &Directory,
    target: &str,
) -> Result<FileSnapshot, DistributionError> {
    match read_file(root, stage_name, FILE_LIMIT) {
        Ok(Some(captured)) => Ok(captured),
        Ok(None) => {
            rollback_swap(root, stage_name, target_parent, target)?;
            Err(error(DistributionErrorId::ObjectChanged))
        }
        Err(failure) => {
            rollback_swap(root, stage_name, target_parent, target)?;
            Err(failure)
        }
    }
}

fn capture_after_rename(
    root: &Directory,
    stage_name: &str,
    target_parent: &Directory,
    target: &str,
) -> Result<FileSnapshot, DistributionError> {
    match read_file(root, stage_name, FILE_LIMIT) {
        Ok(Some(captured)) => Ok(captured),
        Ok(None) => {
            rollback_rename(root, stage_name, target_parent, target)?;
            Err(error(DistributionErrorId::ObjectChanged))
        }
        Err(failure) => {
            rollback_rename(root, stage_name, target_parent, target)?;
            Err(failure)
        }
    }
}

fn replacement_file_current(
    stage: &OwnedFile,
    parent: &Directory,
    name: &str,
    replacement: &[u8],
) -> Result<bool, DistributionError> {
    stage.require_at(parent, name)?;
    Ok(
        read_file(parent, name, FILE_LIMIT)?.is_some_and(|snapshot| {
            snapshot.identity.same_after_rename(stage.identity())
                && snapshot.mode == stage.mode()
                && snapshot.bytes == replacement
        }),
    )
}

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

fn rollback_swap(
    left_parent: &Directory,
    left: &str,
    right_parent: &Directory,
    right: &str,
) -> Result<(), DistributionError> {
    match rename_swap(left_parent, left, right_parent, right) {
        Ok(true) => Ok(()),
        _ => Err(error(DistributionErrorId::RollbackFailed)),
    }
}

pub(super) fn same_snapshot(left: Option<&FileSnapshot>, right: Option<&FileSnapshot>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => left.identity == right.identity && left.bytes == right.bytes,
        _ => false,
    }
}

fn same_moved_snapshot(left: &FileSnapshot, right: &FileSnapshot) -> bool {
    left.identity.same_after_rename(right.identity)
        && left.mode == right.mode
        && left.bytes == right.bytes
}
