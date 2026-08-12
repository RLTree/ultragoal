use super::*;

use std::fs::{self, File};
use std::io::{Seek, SeekFrom, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::routine_work::runtime_adapter::mediator::{
    ObjectIdentity, PinnedExecutable, StagedProgram,
};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[cfg(unix)]
#[path = "snapshot_bound_filesystem.rs"]
mod bound_filesystem;
#[cfg(unix)]
use bound_filesystem::{create_exclusive_at, validate_directory_path};

use super::acquisition::{LaunchAcquisitionCustody, observe_directory_stat};
use super::cleanup::EntryClaim;
use super::root::safe_token_name;

pub(in crate::routine_work::runtime_adapter::production) struct LaunchRoot {
    path: PathBuf,
    file: File,
    identity: ObjectIdentity,
}

pub(in crate::routine_work::runtime_adapter::production) struct LaunchBinding<'a> {
    pub(in crate::routine_work::runtime_adapter::production) grant_id: &'a str,
    pub(in crate::routine_work::runtime_adapter::production) recovery_marker: &'a str,
    pub(in crate::routine_work::runtime_adapter::production) intent_id: &'a str,
}

pub(in crate::routine_work::runtime_adapter::production) fn launch_root(
    custody: &super::super::RoutineCustodyCapability,
) -> Result<LaunchRoot, RoutineError> {
    let (path, file, identity) = custody.launch_directory()?;
    validate_launch_root(&path, &file, identity)?;
    Ok(LaunchRoot {
        path,
        file,
        identity,
    })
}

pub(super) fn validate_launch_root(
    path: &Path,
    file: &File,
    expected: ObjectIdentity,
) -> Result<(), RoutineError> {
    let held = ObjectIdentity::from(
        &file
            .metadata()
            .map_err(|_| error("routine-production-launch-directory-stat-failed"))?,
    );
    if !directory_binding_matches(expected, held) {
        return Err(error("routine-production-launch-directory-mismatch"));
    }
    validate_directory_path(path, expected)
}

pub(in crate::routine_work::runtime_adapter::production) fn stage_program(
    root: &LaunchRoot,
    binding: LaunchBinding<'_>,
    source: &PinnedExecutable,
) -> Result<StagedProgram, RoutineError> {
    #[cfg(not(unix))]
    {
        let _ = (root, binding, source);
        return Err(error("routine-production-launch-unix-required"));
    }
    #[cfg(unix)]
    {
        source.validate()?;
        validate_launch_root(&root.path, &root.file, root.identity)?;
        let base = format!(
            "launch-{}-{}",
            safe_token_name(binding.grant_id),
            safe_token_name(binding.intent_id)
        );
        let marker_name = format!("{base}-authority");
        let program_name = format!("{base}-program");
        let seal_name = format!("{base}-sealed");
        let directory = root
            .file
            .try_clone()
            .map_err(|_| error("routine-production-launch-directory-duplicate-failed"))?;
        let mut custody =
            LaunchAcquisitionCustody::held(root.path.clone(), directory, root.identity);
        let result = catch_unwind(AssertUnwindSafe(
            || -> Result<StagedProgram, RoutineError> {
                observe_directory_stat()?;
                validate_launch_root(custody.root(), &root.file, root.identity)?;
                observe_directory_stat()?;
                let directory = custody.duplicate_directory()?;
                validate_launch_root(custody.root(), &directory, root.identity)?;

                let marker = root.path.join(&marker_name);
                let marker_bytes = format!(
                    "grant:{}\nrecovery:{}\nintent:{}\n",
                    binding.grant_id, binding.recovery_marker, binding.intent_id
                )
                .into_bytes();
                let mut marker_file = create_exclusive_at(&directory, &marker_name, 0o400)
                    .map_err(|_| error("routine-production-launch-marker-create-failed"))?;
                custody.claim(entry_claim(&marker, &marker_file, Some(Vec::new()))?);
                marker_file
                    .write_all(&marker_bytes)
                    .map_err(|_| error("routine-production-launch-marker-write-failed"))?;
                marker_file
                    .sync_all()
                    .map_err(|_| error("routine-production-launch-marker-sync-failed"))?;
                custody.claim(entry_claim(
                    &marker,
                    &marker_file,
                    Some(marker_bytes.clone()),
                )?);

                let path = root.path.join(&program_name);
                let mut destination = create_exclusive_at(&directory, &program_name, 0o500)
                    .map_err(|_| error("routine-production-launch-file-create-failed"))?;
                custody.claim(entry_claim(&path, &destination, None)?);
                let mut source_file = source
                    .file
                    .try_clone()
                    .map_err(|_| error("routine-production-launch-source-duplicate-failed"))?;
                source_file
                    .seek(SeekFrom::Start(0))
                    .map_err(|_| error("routine-production-launch-source-seek-failed"))?;
                std::io::copy(&mut source_file, &mut destination)
                    .map_err(|_| error("routine-production-launch-copy-failed"))?;
                destination
                    .sync_all()
                    .map_err(|_| error("routine-production-launch-sync-failed"))?;
                let staged_mode = source.identity_mode().unwrap_or(0o500) & !0o222;
                destination
                    .set_permissions(fs::Permissions::from_mode(staged_mode))
                    .map_err(|_| error("routine-production-launch-file-mode-failed"))?;
                destination
                    .sync_all()
                    .map_err(|_| error("routine-production-launch-sync-failed"))?;
                custody.claim(entry_claim(&path, &destination, None)?);
                source.validate()?;
                let executable = PinnedExecutable::from_bound_file(
                    &path,
                    destination,
                    &source.sha256,
                    source.identity_length(),
                    staged_mode,
                )?;

                let seal = root.path.join(&seal_name);
                let seal_bytes = format!(
                    "grant:{}\nintent:{}\ndigest:{}\n",
                    binding.grant_id, binding.intent_id, source.sha256
                )
                .into_bytes();
                let mut seal_file = create_exclusive_at(&directory, &seal_name, 0o400)
                    .map_err(|_| error("routine-production-launch-seal-create-failed"))?;
                custody.claim(entry_claim(&seal, &seal_file, Some(Vec::new()))?);
                seal_file
                    .write_all(&seal_bytes)
                    .map_err(|_| error("routine-production-launch-seal-write-failed"))?;
                seal_file
                    .sync_all()
                    .map_err(|_| error("routine-production-launch-seal-sync-failed"))?;
                custody.claim(entry_claim(&seal, &seal_file, Some(seal_bytes.clone()))?);

                directory
                    .sync_all()
                    .map_err(|_| error("routine-production-launch-directory-sync-failed"))?;
                validate_launch_root(&root.path, &directory, root.identity)?;
                executable.validate_bound_at(&directory, &program_name)?;
                let marker_identity = ObjectIdentity::from(
                    &marker_file
                        .metadata()
                        .map_err(|_| error("routine-production-launch-marker-stat-failed"))?,
                );
                let seal_identity = ObjectIdentity::from(
                    &seal_file
                        .metadata()
                        .map_err(|_| error("routine-production-launch-seal-stat-failed"))?,
                );
                Ok(StagedProgram {
                    executable,
                    directory_file: directory,
                    directory: root.path.clone(),
                    marker,
                    seal,
                    marker_bytes,
                    seal_bytes,
                    directory_identity: root.identity,
                    marker_identity,
                    seal_identity,
                })
            },
        ));
        match result {
            Ok(Ok(staged)) => Ok(staged),
            Ok(Err(primary)) => Err(primary.with_launch_cleanup(custody.observe_cleanup())),
            Err(payload) => super::super::reservation_failure::resume_launch_acquisition_panic(
                payload,
                custody.observe_cleanup(),
            ),
        }
    }
}

fn directory_binding_matches(expected: ObjectIdentity, current: ObjectIdentity) -> bool {
    expected.device == current.device
        && expected.inode == current.inode
        && expected.mode == current.mode
        && expected.owner_user_id == current.owner_user_id
        && expected.owner_group_id == current.owner_group_id
}

fn entry_claim(
    path: &Path,
    file: &File,
    bytes: Option<Vec<u8>>,
) -> Result<EntryClaim, RoutineError> {
    Ok(EntryClaim {
        path: path.to_path_buf(),
        identity: ObjectIdentity::from(
            &file
                .metadata()
                .map_err(|_| error("routine-production-launch-entry-stat-failed"))?,
        ),
        bytes,
    })
}
