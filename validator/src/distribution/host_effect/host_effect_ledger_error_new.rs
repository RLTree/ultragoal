impl HostEffectLedgerError {
    pub(in crate::distribution::host_effect) const fn new(id: HostEffectLedgerErrorId) -> Self {
        Self { id }
    }

    #[cfg(test)]
    pub(crate) const fn id(&self) -> HostEffectLedgerErrorId {
        self.id
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct PinnedHostExecutableIdentity {
    canonical_path: String,
    content_sha256: String,
    device: u64,
    inode: u64,
    mode: u32,
    uid: u32,
    gid: u32,
    hard_links: u64,
    size: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

impl PinnedHostExecutableIdentity {
    pub(crate) fn binding_sha256(&self) -> Result<String, HostEffectLedgerError> {
        #[derive(Serialize)]
        struct Binding<'a> {
            schema: &'static str,
            identity: &'a PinnedHostExecutableIdentity,
        }
        serde_json::to_vec(&Binding {
            schema: "harness-ultragoal.pinned-host-executable.v1",
            identity: self,
        })
        .map(|bytes| format!("sha256:{:x}", Sha256::digest(bytes)))
        .map_err(|_| HostEffectLedgerError::new(HostEffectLedgerErrorId::InvalidRecord))
    }
}

/// Owns the open executable object. It is intentionally neither Clone nor
/// serializable; an executor must keep this object alive through child start.
pub(crate) struct PinnedHostExecutable {
    file: File,
    identity: PinnedHostExecutableIdentity,
}

impl PinnedHostExecutable {
    pub(in crate::distribution::host_effect) fn pin(
        path: &Path,
    ) -> Result<Self, HostEffectLedgerError> {
        #[cfg(unix)]
        {
            let canonical = fs::canonicalize(path).map_err(|_| ledger_io())?;
            if !canonical.is_absolute() {
                return Err(invalid_record());
            }
            let mut options = OpenOptions::new();
            options
                .read(true)
                .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK);
            let file = options.open(&canonical).map_err(|_| ledger_io())?;
            let identity = capture_executable_identity(&file, &canonical)?;
            Ok(Self { file, identity })
        }
        #[cfg(not(unix))]
        {
            let _ = path;
            Err(invalid_record())
        }
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    pub(in crate::distribution::host_effect) fn file(&self) -> &File {
        &self.file
    }

    #[cfg(target_os = "macos")]
    pub(in crate::distribution::host_effect) fn loaded_identity(&self) -> (&Path, u64, u64) {
        (
            Path::new(&self.identity.canonical_path),
            self.identity.device,
            self.identity.inode,
        )
    }

    pub(crate) fn identity(&self) -> &PinnedHostExecutableIdentity {
        &self.identity
    }

    pub(in crate::distribution::host_effect) fn duplicate(
        &self,
    ) -> Result<Self, HostEffectLedgerError> {
        Ok(Self {
            file: self.file.try_clone().map_err(|_| ledger_io())?,
            identity: self.identity.clone(),
        })
    }

    pub(in crate::distribution::host_effect) fn revalidate(
        &self,
    ) -> Result<(), HostEffectLedgerError> {
        #[cfg(unix)]
        {
            let current =
                capture_executable_identity(&self.file, Path::new(&self.identity.canonical_path))?;
            if current != self.identity {
                return Err(HostEffectLedgerError::new(
                    HostEffectLedgerErrorId::Tampered,
                ));
            }
            Ok(())
        }
        #[cfg(not(unix))]
        {
            Err(invalid_record())
        }
    }
}

#[cfg(unix)]
fn capture_executable_identity(
    file: &File,
    canonical_path: &Path,
) -> Result<PinnedHostExecutableIdentity, HostEffectLedgerError> {
    let path_before = fs::symlink_metadata(canonical_path).map_err(|_| ledger_io())?;
    let descriptor_before = file.metadata().map_err(|_| ledger_io())?;
    validate_executable_metadata(&path_before)?;
    validate_executable_metadata(&descriptor_before)?;
    if !same_executable_object(&path_before, &descriptor_before) {
        return Err(tampered());
    }

    let mut hasher = Sha256::new();
    let mut offset = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    while offset < descriptor_before.len() {
        let remaining = (descriptor_before.len() - offset).min(buffer.len() as u64) as usize;
        let count = read_at_retry(file, &mut buffer[..remaining], offset)?;
        if count == 0 {
            return Err(tampered());
        }
        hasher.update(&buffer[..count]);
        offset = offset
            .checked_add(count as u64)
            .ok_or_else(invalid_record)?;
    }
    let mut extra = [0_u8; 1];
    if read_at_retry(file, &mut extra, descriptor_before.len())? != 0 {
        return Err(tampered());
    }

    let descriptor_after = file.metadata().map_err(|_| ledger_io())?;
    let path_after = fs::symlink_metadata(canonical_path).map_err(|_| ledger_io())?;
    validate_executable_metadata(&descriptor_after)?;
    validate_executable_metadata(&path_after)?;
    if !same_executable_object(&descriptor_before, &descriptor_after)
        || !same_executable_object(&descriptor_after, &path_after)
        || fs::canonicalize(canonical_path).map_err(|_| ledger_io())? != canonical_path
    {
        return Err(tampered());
    }
    let canonical_path = canonical_path
        .to_str()
        .ok_or_else(invalid_record)?
        .to_owned();
    Ok(PinnedHostExecutableIdentity {
        canonical_path,
        content_sha256: format!("sha256:{:x}", hasher.finalize()),
        device: descriptor_after.dev(),
        inode: descriptor_after.ino(),
        mode: descriptor_after.mode(),
        uid: descriptor_after.uid(),
        gid: descriptor_after.gid(),
        hard_links: descriptor_after.nlink(),
        size: descriptor_after.len(),
        modified_seconds: descriptor_after.mtime(),
        modified_nanoseconds: descriptor_after.mtime_nsec(),
        changed_seconds: descriptor_after.ctime(),
        changed_nanoseconds: descriptor_after.ctime_nsec(),
    })
}

#[cfg(unix)]
fn validate_executable_metadata(metadata: &fs::Metadata) -> Result<(), HostEffectLedgerError> {
    if !metadata.file_type().is_file()
        || metadata.len() == 0
        || metadata.len() > MAX_PINNED_EXECUTABLE_BYTES
        || metadata.nlink() != 1
        || metadata.mode() & 0o111 == 0
    {
        return Err(invalid_record());
    }
    Ok(())
}
