#[derive(Clone, Debug)]
struct PinnedRuntimeExecutable {
    path: PathBuf,
    file: std::sync::Arc<std::fs::File>,
    identity: RuntimeExecutableIdentity,
    sha256: String,
    shell_script: bool,
}

#[cfg(unix)]
#[derive(Clone, Debug, Eq, PartialEq)]
struct RuntimeExecutableIdentity {
    dev: u64,
    ino: u64,
    mode: u32,
    uid: u32,
    gid: u32,
    nlink: u64,
    len: u64,
    mtime: i64,
    mtime_nsec: i64,
    ctime: i64,
    ctime_nsec: i64,
}

#[cfg(not(unix))]
#[derive(Clone, Debug, Eq, PartialEq)]
struct RuntimeExecutableIdentity;

impl PinnedRuntimeExecutable {
    fn open(path: &Path) -> Result<Self, DistributionError> {
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            use std::os::unix::fs::OpenOptionsExt;

            let canonical = path
                .canonicalize()
                .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
            let mut options = std::fs::OpenOptions::new();
            options
                .read(true)
                .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
            let file = options
                .open(&canonical)
                .map_err(|_| error(DistributionErrorId::UnsafeObject))?;
            clear_close_on_exec(file.as_raw_fd())?;
            let (identity, sha256, shell_script) = capture_runtime_executable(&file, &canonical)?;
            Ok(Self {
                path: canonical,
                file: std::sync::Arc::new(file),
                identity,
                sha256,
                shell_script,
            })
        }
        #[cfg(not(unix))]
        {
            let _ = path;
            Err(error(DistributionErrorId::CapabilityMismatch))
        }
    }

    fn sha256(&self) -> &str {
        &self.sha256
    }

    fn shell_script(&self) -> bool {
        self.shell_script
    }

    fn revalidate(&self) -> Result<(), DistributionError> {
        #[cfg(unix)]
        {
            let (identity, sha256, shell_script) =
                capture_runtime_executable(&self.file, &self.path)?;
            if identity != self.identity || sha256 != self.sha256 {
                return Err(error(DistributionErrorId::ObjectChanged));
            }
            if shell_script != self.shell_script {
                return Err(error(DistributionErrorId::ObjectChanged));
            }
            Ok(())
        }
        #[cfg(not(unix))]
        {
            Err(error(DistributionErrorId::CapabilityMismatch))
        }
    }
}

#[cfg(unix)]
fn clear_close_on_exec(fd: std::os::fd::RawFd) -> Result<(), DistributionError> {
    // SAFETY: `fd` is borrowed from an open File and F_GETFD has no pointer arguments.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
    if flags < 0 {
        return Err(error(DistributionErrorId::UnsafeObject));
    }
    // SAFETY: `fd` is borrowed from an open File and the flag value is valid for F_SETFD.
    if unsafe { libc::fcntl(fd, libc::F_SETFD, flags & !libc::FD_CLOEXEC) } < 0 {
        return Err(error(DistributionErrorId::UnsafeObject));
    }
    Ok(())
}

#[cfg(unix)]
fn capture_runtime_executable(
    file: &std::fs::File,
    path: &Path,
) -> Result<(RuntimeExecutableIdentity, String, bool), DistributionError> {
    let path_before =
        std::fs::symlink_metadata(path).map_err(|_| error(DistributionErrorId::ObjectChanged))?;
    let descriptor_before = file
        .metadata()
        .map_err(|_| error(DistributionErrorId::UnsafeObject))?;
    validate_runtime_executable(&path_before)?;
    if !same_runtime_object(&path_before, &descriptor_before) {
        return Err(error(DistributionErrorId::ObjectChanged));
    }
    validate_runtime_executable(&descriptor_before)?;
    let sha256 = digest_runtime_executable(file, descriptor_before.len())?;
    let shell_script = runtime_executable_is_shell_script(file)?;
    let descriptor_after = file
        .metadata()
        .map_err(|_| error(DistributionErrorId::UnsafeObject))?;
    let path_after =
        std::fs::symlink_metadata(path).map_err(|_| error(DistributionErrorId::ObjectChanged))?;
    validate_runtime_executable(&path_after)?;
    if !same_runtime_object(&descriptor_before, &descriptor_after)
        || !same_runtime_object(&descriptor_after, &path_after)
        || path
            .canonicalize()
            .map_err(|_| error(DistributionErrorId::ObjectChanged))?
            != path
    {
        return Err(error(DistributionErrorId::ObjectChanged));
    }
    validate_runtime_executable(&descriptor_after)?;
    Ok((identity_from(&descriptor_after), sha256, shell_script))
}

#[cfg(unix)]
fn validate_runtime_executable(metadata: &std::fs::Metadata) -> Result<(), DistributionError> {
    use std::os::unix::fs::MetadataExt;
    if !metadata.file_type().is_file()
        || metadata.len() == 0
        || metadata.len() > EXECUTABLE_LIMIT as u64
        || metadata.nlink() != 1
        || metadata.mode() & 0o111 == 0
    {
        return Err(error(DistributionErrorId::UnsafeObject));
    }
    Ok(())
}

#[cfg(unix)]
fn same_runtime_object(left: &std::fs::Metadata, right: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    left.dev() == right.dev()
        && left.ino() == right.ino()
        && left.mode() == right.mode()
        && left.uid() == right.uid()
        && left.gid() == right.gid()
        && left.nlink() == right.nlink()
        && left.len() == right.len()
        && left.mtime() == right.mtime()
        && left.mtime_nsec() == right.mtime_nsec()
        && left.ctime() == right.ctime()
        && left.ctime_nsec() == right.ctime_nsec()
}

#[cfg(unix)]
fn identity_from(metadata: &std::fs::Metadata) -> RuntimeExecutableIdentity {
    use std::os::unix::fs::MetadataExt;
    RuntimeExecutableIdentity {
        dev: metadata.dev(),
        ino: metadata.ino(),
        mode: metadata.mode(),
        uid: metadata.uid(),
        gid: metadata.gid(),
        nlink: metadata.nlink(),
        len: metadata.len(),
        mtime: metadata.mtime(),
        mtime_nsec: metadata.mtime_nsec(),
        ctime: metadata.ctime(),
        ctime_nsec: metadata.ctime_nsec(),
    }
}

#[cfg(unix)]
fn digest_runtime_executable(file: &std::fs::File, len: u64) -> Result<String, DistributionError> {
    use std::os::unix::fs::FileExt;
    let mut offset = 0_u64;
    let mut bytes = Vec::with_capacity(len as usize);
    let mut buffer = [0_u8; 64 * 1024];
    while offset < len {
        let remaining = (len - offset).min(buffer.len() as u64) as usize;
        let count = file
            .read_at(&mut buffer[..remaining], offset)
            .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
        if count == 0 {
            return Err(error(DistributionErrorId::ObjectChanged));
        }
        bytes.extend_from_slice(&buffer[..count]);
        offset = offset
            .checked_add(count as u64)
            .ok_or_else(|| error(DistributionErrorId::ObjectTooLarge))?;
    }
    let mut extra = [0_u8; 1];
    if file
        .read_at(&mut extra, len)
        .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?
        != 0
    {
        return Err(error(DistributionErrorId::ObjectChanged));
    }
    Ok(sha256(&bytes))
}

#[cfg(unix)]
fn runtime_executable_is_shell_script(file: &std::fs::File) -> Result<bool, DistributionError> {
    use std::os::unix::fs::FileExt;
    let mut prefix = [0_u8; 10];
    let count = file
        .read_at(&mut prefix, 0)
        .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
    Ok(count >= 10 && &prefix == b"#!/bin/sh\n")
}
