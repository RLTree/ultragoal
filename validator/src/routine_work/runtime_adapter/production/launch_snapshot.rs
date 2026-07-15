use super::*;

use std::fs::{self, File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};

use crate::routine_work::runtime_adapter::mediator::{
    ObjectIdentity, PinnedExecutable, StagedProgram,
};
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

use super::launch_custody::{EntryClaim, cleanup_created_child, cleanup_partial_stage};
use super::launch_root::{ensure_launch_root, safe_token_name};

pub(super) const LAUNCH_ROOT_NAME: &str = ".routine-authority-launch";
pub(super) const LAUNCH_FILE_NAME: &str = "program";
pub(super) const LAUNCH_MARKER_NAME: &str = "authority";
pub(super) const LAUNCH_SEAL_NAME: &str = "sealed";

pub(super) fn launch_root(authority_root: &Path) -> Result<PathBuf, RoutineError> {
    let parent = authority_root
        .parent()
        .ok_or_else(|| error("routine-production-launch-root-invalid"))?
        .canonicalize()
        .map_err(|_| error("routine-production-launch-root-unavailable"))?;
    Ok(parent.join(LAUNCH_ROOT_NAME))
}

pub(super) fn stage_program(
    root: &Path,
    token: &ReservationToken,
    source: &PinnedExecutable,
) -> Result<StagedProgram, RoutineError> {
    #[cfg(not(unix))]
    {
        let _ = (root, token, source);
        return Err(error("routine-production-launch-unix-required"));
    }
    #[cfg(unix)]
    {
        source.validate()?;
        ensure_launch_root(root)?;
        let child = root.join(format!("launch-{}", safe_token_name(&token.grant_id)));
        fs::create_dir(&child)
            .map_err(|_| error("routine-production-launch-directory-create-failed"))?;
        let created_directory_identity = ObjectIdentity::from(
            &fs::symlink_metadata(&child)
                .map_err(|_| error("routine-production-launch-directory-stat-failed"))?,
        );
        if let Err(value) = fs::set_permissions(&child, fs::Permissions::from_mode(0o700)) {
            return Err(cleanup_created_child(
                &child,
                created_directory_identity,
                value,
                "routine-production-launch-directory-mode-failed",
            ));
        }
        let directory = match File::open(&child) {
            Ok(directory) => directory,
            Err(value) => {
                return Err(cleanup_created_child(
                    &child,
                    created_directory_identity,
                    value,
                    "routine-production-launch-directory-open-failed",
                ));
            }
        };
        let directory_identity = ObjectIdentity::from(
            &directory
                .metadata()
                .map_err(|_| error("routine-production-launch-directory-stat-failed"))?,
        );
        let marker = child.join(LAUNCH_MARKER_NAME);
        let marker_bytes = format!("{}\n{}\n", token.grant_id, token.recovery_marker).into_bytes();
        let mut marker_file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .mode(0o400)
            .open(&marker)
        {
            Ok(file) => file,
            Err(_) => {
                cleanup_partial_stage(&child, directory_identity, &[])?;
                return Err(error("routine-production-launch-marker-create-failed"));
            }
        };
        let marker_created_identity = match marker_file.metadata() {
            Ok(metadata) => ObjectIdentity::from(&metadata),
            Err(_) => {
                return Err(
                    match cleanup_partial_stage(&child, directory_identity, &[]) {
                        Ok(()) => error("routine-production-launch-marker-stat-failed"),
                        Err(cleanup_error) => cleanup_error,
                    },
                );
            }
        };
        let marker_claim = EntryClaim {
            path: marker.clone(),
            identity: marker_created_identity,
            bytes: Some(marker_bytes.clone()),
        };
        let mut program_claim = None;
        let mut seal_claim = None;
        let result = (|| {
            marker_file
                .write_all(&marker_bytes)
                .map_err(|_| error("routine-production-launch-marker-write-failed"))?;
            marker_file
                .sync_all()
                .map_err(|_| error("routine-production-launch-marker-sync-failed"))?;

            let path = child.join(LAUNCH_FILE_NAME);
            let mut destination = OpenOptions::new()
                .write(true)
                .create_new(true)
                .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .mode(0o500)
                .open(&path)
                .map_err(|_| error("routine-production-launch-file-create-failed"))?;
            let program_identity = ObjectIdentity::from(
                &destination
                    .metadata()
                    .map_err(|_| error("routine-production-launch-file-stat-failed"))?,
            );
            program_claim = Some(EntryClaim {
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
            fs::set_permissions(&path, fs::Permissions::from_mode(staged_mode))
                .map_err(|_| error("routine-production-launch-file-mode-failed"))?;
            source.validate()?;
            let executable = PinnedExecutable::open_bound_path(
                &path,
                &source.sha256,
                source.identity_length(),
                Some(staged_mode),
            )?;
            executable.validate()?;
            let seal = child.join(LAUNCH_SEAL_NAME);
            let seal_bytes = format!("{}\n", source.sha256).into_bytes();
            let mut seal_file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .mode(0o400)
                .open(&seal)
                .map_err(|_| error("routine-production-launch-seal-create-failed"))?;
            let seal_created_identity = ObjectIdentity::from(
                &seal_file
                    .metadata()
                    .map_err(|_| error("routine-production-launch-seal-stat-failed"))?,
            );
            seal_claim = Some(EntryClaim {
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
            Ok(StagedProgram {
                executable,
                directory: child.clone(),
                marker: marker.clone(),
                seal,
                marker_bytes,
                seal_bytes,
                directory_identity,
                marker_identity,
                seal_identity,
            })
        })();
        match result {
            Ok(staged) => Ok(staged),
            Err(error) => {
                let mut claims = vec![EntryClaim {
                    path: marker_claim.path.clone(),
                    identity: marker_claim.identity,
                    bytes: marker_claim.bytes.clone(),
                }];
                if let Some(claim) = program_claim {
                    claims.push(claim);
                }
                if let Some(claim) = seal_claim {
                    claims.push(claim);
                }
                match cleanup_partial_stage(&child, directory_identity, &claims) {
                    Ok(()) => Err(error),
                    Err(cleanup_error) => Err(cleanup_error),
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "launch_snapshot_tests.rs"]
mod tests;
