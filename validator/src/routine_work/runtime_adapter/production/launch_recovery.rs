use super::*;

use std::fs;

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};

use crate::routine_work::runtime_adapter::execution_authority::RoutineEffectIntent;
use crate::routine_work::runtime_adapter::mediator::{
    ObjectIdentity, PinnedExecutable, StagedProgram,
};

use super::launch_custody::{EntryClaim, cleanup_partial_stage, cleanup_staged};
use super::launch_snapshot::{LAUNCH_FILE_NAME, LAUNCH_MARKER_NAME, LAUNCH_SEAL_NAME};
use super::launch_stage_support::{ensure_launch_root, safe_token_name};

pub(super) fn recover_staged(
    root: &Path,
    grant_id: &str,
    recovery_marker: &str,
    intents: &[RoutineEffectIntent],
) -> Result<(), RoutineError> {
    #[cfg(not(unix))]
    {
        let _ = (root, grant_id, recovery_marker, intents);
        return Err(error("routine-production-launch-unix-required"));
    }
    #[cfg(unix)]
    {
        ensure_launch_root(root)?;
        let child = root.join(format!("launch-{}", safe_token_name(grant_id)));
        let directory_metadata = match fs::symlink_metadata(&child) {
            Ok(metadata) => metadata,
            Err(value) if value.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(_) => return Err(error("routine-production-launch-directory-stat-failed")),
        };
        if directory_metadata.file_type().is_symlink()
            || !directory_metadata.is_dir()
            || directory_metadata.uid() != unsafe { libc::geteuid() }
            || directory_metadata.permissions().mode() != 0o40700
        {
            return Err(error("routine-production-launch-directory-unsafe"));
        }
        let directory = fs::File::open(&child)
            .map_err(|_| error("routine-production-launch-directory-open-failed"))?;
        let directory_identity = ObjectIdentity::from(
            &directory
                .metadata()
                .map_err(|_| error("routine-production-launch-directory-stat-failed"))?,
        );
        if directory_identity != ObjectIdentity::from(&directory_metadata) {
            return Err(error("routine-production-launch-directory-mismatch"));
        }
        let entries = fs::read_dir(&child)
            .map_err(|_| error("routine-production-launch-directory-read-failed"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| error("routine-production-launch-directory-read-failed"))?;
        if entries.iter().any(|entry| {
            !matches!(
                entry.file_name().to_str(),
                Some(LAUNCH_MARKER_NAME | LAUNCH_FILE_NAME | LAUNCH_SEAL_NAME)
            )
        }) {
            return Err(error(
                "routine-production-launch-directory-contents-invalid",
            ));
        }

        let marker = child.join(LAUNCH_MARKER_NAME);
        let marker_metadata = match fs::symlink_metadata(&marker) {
            Ok(metadata) => Some(metadata),
            Err(value) if value.kind() == std::io::ErrorKind::NotFound => None,
            Err(_) => return Err(error("routine-production-launch-marker-stat-failed")),
        };
        if marker_metadata.is_none() {
            if !entries.is_empty() {
                return Err(error("routine-production-launch-marker-missing"));
            }
            return cleanup_partial_stage(&child, directory_identity, &[]);
        }
        let Some(marker_metadata) = marker_metadata else {
            return Err(error("routine-production-launch-marker-missing"));
        };
        if marker_metadata.file_type().is_symlink()
            || !marker_metadata.is_file()
            || marker_metadata.nlink() != 1
            || marker_metadata.uid() != unsafe { libc::geteuid() }
            || marker_metadata.permissions().mode() & 0o222 != 0
        {
            return Err(error("routine-production-launch-marker-mismatch"));
        }
        let marker_bytes =
            fs::read(&marker).map_err(|_| error("routine-production-launch-marker-read-failed"))?;
        let expected_marker = format!("{}\n{}\n", grant_id, recovery_marker).into_bytes();
        if marker_bytes != expected_marker {
            return Err(error("routine-production-launch-marker-mismatch"));
        }
        let marker_identity = ObjectIdentity::from(&marker_metadata);

        let program = child.join(LAUNCH_FILE_NAME);
        let program_metadata = match fs::symlink_metadata(&program) {
            Ok(metadata) => Some(metadata),
            Err(value) if value.kind() == std::io::ErrorKind::NotFound => None,
            Err(_) => return Err(error("routine-production-launch-file-stat-failed")),
        };
        let Some(program_metadata) = program_metadata else {
            if entries.len() != 1 {
                return Err(error(
                    "routine-production-launch-directory-contents-invalid",
                ));
            }
            return cleanup_partial_stage(
                &child,
                directory_identity,
                &[EntryClaim {
                    path: marker,
                    identity: marker_identity,
                    bytes: Some(expected_marker),
                }],
            );
        };
        if program_metadata.file_type().is_symlink()
            || !program_metadata.is_file()
            || program_metadata.nlink() != 1
            || program_metadata.uid() != unsafe { libc::geteuid() }
            || program_metadata.permissions().mode() & 0o222 != 0
            || program_metadata.permissions().mode() & 0o111 == 0
        {
            return Err(error("routine-production-launch-file-mismatch"));
        }
        let program_identity = ObjectIdentity::from(&program_metadata);

        let seal = child.join(LAUNCH_SEAL_NAME);
        let seal_metadata = match fs::symlink_metadata(&seal) {
            Ok(metadata) => Some(metadata),
            Err(value) if value.kind() == std::io::ErrorKind::NotFound => None,
            Err(_) => return Err(error("routine-production-launch-seal-stat-failed")),
        };
        let Some(seal_metadata) = seal_metadata else {
            if entries.len() != 2 {
                return Err(error(
                    "routine-production-launch-directory-contents-invalid",
                ));
            }
            return cleanup_partial_stage(
                &child,
                directory_identity,
                &[
                    EntryClaim {
                        path: program,
                        identity: program_identity,
                        bytes: None,
                    },
                    EntryClaim {
                        path: marker,
                        identity: marker_identity,
                        bytes: Some(expected_marker),
                    },
                ],
            );
        };
        if seal_metadata.file_type().is_symlink()
            || !seal_metadata.is_file()
            || seal_metadata.nlink() != 1
            || seal_metadata.uid() != unsafe { libc::geteuid() }
            || seal_metadata.permissions().mode() & 0o222 != 0
        {
            return Err(error("routine-production-launch-seal-mismatch"));
        }
        if entries.len() != 3 {
            return Err(error(
                "routine-production-launch-directory-contents-invalid",
            ));
        }
        let seal_bytes =
            fs::read(&seal).map_err(|_| error("routine-production-launch-seal-read-failed"))?;
        let executable = PinnedExecutable::open_unbound(&program)
            .map_err(|_| error("routine-production-launch-sealed-file-invalid"))?;
        let expected_seal = format!("{}\n", executable.sha256).into_bytes();
        if seal_bytes != expected_seal || executable.identity != program_identity {
            return Err(error("routine-production-launch-seal-mismatch"));
        }
        if !intents.iter().any(|intent| {
            intent.program_sha256() == executable.sha256
                && intent.program_byte_length() == executable.identity_length()
        }) {
            return Err(error("routine-production-launch-binding-mismatch"));
        }
        cleanup_staged(&StagedProgram {
            executable,
            directory: child,
            marker,
            seal,
            marker_bytes: expected_marker,
            seal_bytes,
            directory_identity,
            marker_identity,
            seal_identity: ObjectIdentity::from(&seal_metadata),
        })
    }
}

#[cfg(test)]
#[path = "launch_recovery_tests.rs"]
mod tests;
