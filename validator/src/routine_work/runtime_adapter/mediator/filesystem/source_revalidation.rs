use super::*;

#[cfg(unix)]
pub(crate) fn source_matches(
    record: &RoutineReadSource,
    identity: ObjectIdentity,
    digest: &str,
) -> bool {
    record.device == identity.device
        && record.inode == identity.inode
        && record.unix_mode == identity.mode
        && record.owner_user_id == identity.owner_user_id
        && record.owner_group_id == identity.owner_group_id
        && record.link_count == identity.links
        && record.byte_length == identity.length
        && record.modified_seconds == identity.modified_seconds
        && record.modified_nanos == identity.modified_nanos
        && record.changed_seconds == identity.changed_seconds
        && record.changed_nanos == identity.changed_nanos
        && record.sha256 == digest
}

#[cfg(unix)]
pub(crate) fn directory_object_matches(expected: ObjectIdentity, current: ObjectIdentity) -> bool {
    expected.device == current.device
        && expected.inode == current.inode
        && expected.mode == current.mode
        && expected.owner_user_id == current.owner_user_id
        && expected.owner_group_id == current.owner_group_id
}

pub(crate) struct RootAnchor {
    pub(crate) path: PathBuf,
    pub(crate) file: File,
    #[cfg(unix)]
    pub(crate) identity: ObjectIdentity,
}

impl RootAnchor {
    pub(crate) fn open(path: &Path) -> Result<Self, RoutineError> {
        #[cfg(not(unix))]
        {
            let _ = path;
            return Err(mediator_error("mediator-unix-confinement-required"));
        }
        #[cfg(unix)]
        {
            let lexical = path.to_path_buf();
            let metadata = fs::symlink_metadata(&lexical)
                .map_err(|_| mediator_error("mediator-working-directory-unavailable"))?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(mediator_error(
                    "mediator-working-directory-identity-invalid",
                ));
            }
            let canonical = lexical
                .canonicalize()
                .map_err(|_| mediator_error("mediator-working-directory-unavailable"))?;
            if canonical != lexical {
                return Err(mediator_error("mediator-working-directory-not-canonical"));
            }
            let file = OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .open(&canonical)
                .map_err(|_| mediator_error("mediator-working-directory-open-failed"))?;
            let identity = ObjectIdentity::from(
                &file
                    .metadata()
                    .map_err(|_| mediator_error("mediator-working-directory-metadata-failed"))?,
            );
            Ok(Self {
                path: canonical,
                file,
                identity,
            })
        }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    #[cfg(unix)]
    pub(crate) fn raw_fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }

    #[cfg(unix)]
    pub(crate) fn device(&self) -> u64 {
        self.identity.device
    }

    pub(crate) fn validate(&self) -> Result<(), RoutineError> {
        #[cfg(not(unix))]
        {
            return Err(mediator_error("mediator-unix-confinement-required"));
        }
        #[cfg(unix)]
        {
            let held = ObjectIdentity::from(
                &self
                    .file
                    .metadata()
                    .map_err(|_| mediator_error("mediator-working-directory-metadata-failed"))?,
            );
            let current_meta = fs::symlink_metadata(&self.path)
                .map_err(|_| mediator_error("mediator-working-directory-replaced"))?;
            if current_meta.file_type().is_symlink() || !current_meta.is_dir() {
                return Err(mediator_error("mediator-working-directory-replaced"));
            }
            let current = ObjectIdentity::from(&current_meta);
            let canonical = self
                .path
                .canonicalize()
                .map_err(|_| mediator_error("mediator-working-directory-replaced"))?;
            if !directory_object_matches(self.identity, held)
                || !directory_object_matches(self.identity, current)
                || canonical != self.path
            {
                return Err(RoutineError::new(
                    RoutineErrorId::ConcurrentMutation,
                    "mediator-working-directory-replaced",
                    None,
                ));
            }
            Ok(())
        }
    }
}

pub(crate) struct PinnedExecutable {
    pub(crate) path: PathBuf,
    pub(crate) file: File,
    #[cfg(unix)]
    pub(crate) identity: ObjectIdentity,
    pub(crate) sha256: String,
}

/// An authority-owned execution snapshot. The child only receives the named
/// snapshot path; the parent retains the descriptor and must explicitly clean
/// the private directory after mediation has determined its outcome.
pub(crate) struct StagedProgram {
    pub(crate) executable: PinnedExecutable,
    pub(crate) directory_file: File,
    pub(crate) directory: PathBuf,
    pub(crate) marker: PathBuf,
    pub(crate) seal: PathBuf,
    pub(crate) marker_bytes: Vec<u8>,
    pub(crate) seal_bytes: Vec<u8>,
    #[cfg(unix)]
    pub(crate) directory_identity: ObjectIdentity,
    #[cfg(unix)]
    pub(crate) marker_identity: ObjectIdentity,
    #[cfg(unix)]
    pub(crate) seal_identity: ObjectIdentity,
}
