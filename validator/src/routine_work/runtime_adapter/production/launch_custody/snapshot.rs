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
use bound_filesystem::{create_bound_directory_at, create_exclusive_at, validate_directory_path};

use super::acquisition::{LaunchAcquisitionCustody, observe_directory_stat};
use super::cleanup::EntryClaim;
use super::root::{ensure_launch_root, safe_token_name};

pub(super) const LAUNCH_ROOT_NAME: &str = ".routine-authority-launch";
pub(super) const LAUNCH_FILE_NAME: &str = "program";
pub(super) const LAUNCH_MARKER_NAME: &str = "authority";
pub(super) const LAUNCH_SEAL_NAME: &str = "sealed";

pub(in crate::routine_work::runtime_adapter::production) struct LaunchBinding<'a> {
    pub(in crate::routine_work::runtime_adapter::production) grant_id: &'a str,
    pub(in crate::routine_work::runtime_adapter::production) recovery_marker: &'a str,
}

pub(in crate::routine_work::runtime_adapter::production) fn launch_root(
    custody: &super::super::RoutineCustodyCapability,
) -> Result<PathBuf, RoutineError> {
    let authority_root = custody.authority_root();
    let parent = authority_root
        .parent()
        .ok_or_else(|| error("routine-production-launch-root-invalid"))?
        .canonicalize()
        .map_err(|_| error("routine-production-launch-root-unavailable"))?;
    Ok(parent.join(LAUNCH_ROOT_NAME))
}

pub(in crate::routine_work::runtime_adapter::production) fn stage_program(
    root: &Path,
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
        ensure_launch_root(root)?;
        let child_name = format!("launch-{}", safe_token_name(binding.grant_id));
        let child = root.join(&child_name);
        let held_directory = create_bound_directory_at(root, &child_name)
            .map_err(|_| error("routine-production-launch-directory-binding-unavailable"))?;
        let mut custody = LaunchAcquisitionCustody::created(child);
        custody.hold_directory(held_directory);
        let result = catch_unwind(AssertUnwindSafe(|| {
            let child = custody.child().to_path_buf();
            let directory = custody.duplicate_directory()?;
            observe_directory_stat()?;
            let created_directory_identity = ObjectIdentity::from(
                &fs::symlink_metadata(&child)
                    .map_err(|_| error("routine-production-launch-directory-stat-failed"))?,
            );
            observe_directory_stat()?;
            let directory_identity = ObjectIdentity::from(
                &directory
                    .metadata()
                    .map_err(|_| error("routine-production-launch-directory-stat-failed"))?,
            );
            if created_directory_identity != directory_identity {
                return Err(error("routine-production-launch-directory-mismatch"));
            }
            custody.bind_directory(directory_identity);
            let marker = child.join(LAUNCH_MARKER_NAME);
            let marker_bytes =
                format!("{}\n{}\n", binding.grant_id, binding.recovery_marker).into_bytes();
            let mut marker_file = create_exclusive_at(&directory, LAUNCH_MARKER_NAME, 0o400)
                .map_err(|_| error("routine-production-launch-marker-create-failed"))?;
            let marker_created_identity = ObjectIdentity::from(
                &marker_file
                    .metadata()
                    .map_err(|_| error("routine-production-launch-marker-stat-failed"))?,
            );
            custody.claim(EntryClaim {
                path: marker.clone(),
                identity: marker_created_identity,
                bytes: Some(marker_bytes.clone()),
            });
            marker_file
                .write_all(&marker_bytes)
                .map_err(|_| error("routine-production-launch-marker-write-failed"))?;
            marker_file
                .sync_all()
                .map_err(|_| error("routine-production-launch-marker-sync-failed"))?;

            let path = child.join(LAUNCH_FILE_NAME);
            let mut destination = create_exclusive_at(&directory, LAUNCH_FILE_NAME, 0o500)
                .map_err(|_| error("routine-production-launch-file-create-failed"))?;
            let program_identity = ObjectIdentity::from(
                &destination
                    .metadata()
                    .map_err(|_| error("routine-production-launch-file-stat-failed"))?,
            );
            custody.claim(EntryClaim {
                path: path.clone(),
                identity: program_identity,
                bytes: None,
            });
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
            source.validate()?;
            let executable = PinnedExecutable::from_bound_file(
                &path,
                destination,
                &source.sha256,
                source.identity_length(),
                staged_mode,
            )?;
            let seal = child.join(LAUNCH_SEAL_NAME);
            let seal_bytes = format!("{}\n", source.sha256).into_bytes();
            let mut seal_file = create_exclusive_at(&directory, LAUNCH_SEAL_NAME, 0o400)
                .map_err(|_| error("routine-production-launch-seal-create-failed"))?;
            let seal_created_identity = ObjectIdentity::from(
                &seal_file
                    .metadata()
                    .map_err(|_| error("routine-production-launch-seal-stat-failed"))?,
            );
            custody.claim(EntryClaim {
                path: seal.clone(),
                identity: seal_created_identity,
                bytes: Some(seal_bytes.clone()),
            });
            seal_file
                .write_all(&seal_bytes)
                .map_err(|_| error("routine-production-launch-seal-write-failed"))?;
            seal_file
                .sync_all()
                .map_err(|_| error("routine-production-launch-seal-sync-failed"))?;
            let marker_identity = ObjectIdentity::from(
                &marker_file
                    .metadata()
                    .map_err(|_| error("routine-production-launch-marker-stat-failed"))?,
            );
            directory
                .sync_all()
                .map_err(|_| error("routine-production-launch-directory-sync-failed"))?;
            let directory_identity = ObjectIdentity::from(
                &directory
                    .metadata()
                    .map_err(|_| error("routine-production-launch-directory-stat-failed"))?,
            );
            let seal_identity = ObjectIdentity::from(
                &seal_file
                    .metadata()
                    .map_err(|_| error("routine-production-launch-seal-stat-failed"))?,
            );
            validate_directory_path(&child, directory_identity)?;
            executable.validate_bound_at(&directory, LAUNCH_FILE_NAME)?;
            Ok(StagedProgram {
                executable,
                directory_file: directory,
                directory: child.clone(),
                marker: marker.clone(),
                seal,
                marker_bytes,
                seal_bytes,
                directory_identity,
                marker_identity,
                seal_identity,
            })
        }));
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
