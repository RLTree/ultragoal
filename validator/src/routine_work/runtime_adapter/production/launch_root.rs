use super::*;

use std::fs;

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};

use crate::routine_work::digest::sha256;

#[cfg(unix)]
pub(super) fn ensure_launch_root(root: &Path) -> Result<(), RoutineError> {
    match fs::symlink_metadata(root) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink()
                || !metadata.is_dir()
                || metadata.uid() != unsafe { libc::geteuid() }
                || metadata.permissions().mode() != 0o40700
                || root.canonicalize().ok().as_deref() != Some(root)
            {
                return Err(error("routine-production-launch-root-unsafe"));
            }
        }
        Err(_) => {
            fs::create_dir(root)
                .map_err(|_| error("routine-production-launch-root-create-failed"))?;
            fs::set_permissions(root, fs::Permissions::from_mode(0o700))
                .map_err(|_| error("routine-production-launch-root-mode-failed"))?;
        }
    }
    Ok(())
}

pub(super) fn safe_token_name(value: &str) -> String {
    sha256(value.as_bytes())
        .trim_start_matches("sha256:")
        .to_owned()
}
