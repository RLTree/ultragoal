use super::*;
use getrandom::fill;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub(crate) enum LedgerEffectRequest<'a> {
    Authenticate {
        key: &'a [u8],
        bytes: &'a [u8],
    },
    TemporaryName,
    Rename {
        directory: &'a File,
        from: &'a str,
        to: &'a str,
    },
    Unlink {
        directory: &'a File,
        name: &'a str,
    },
}

pub(crate) enum LedgerEffectResponse {
    Digest(String),
    Name(String),
    Applied,
}

#[derive(Debug)]
pub(crate) struct LedgerEffectError {
    pub(crate) cause: RoutineError,
}

pub(crate) fn hmac(key: &[u8], bytes: &[u8]) -> Result<String, RoutineError> {
    let mut mac = HmacSha256::new_from_slice(key)
        .map_err(|_| error("routine-production-authority-hmac-invalid"))?;
    mac.update(bytes);
    Ok(format!("sha256:{:x}", mac.finalize().into_bytes()))
}

pub(crate) fn read_key(file: &File) -> Result<LedgerKey, RoutineError> {
    let bytes = read_bounded(file, KEY_BYTES as u64)?;
    let value: [u8; KEY_BYTES] = bytes
        .try_into()
        .map_err(|_| error("routine-production-authority-key-size-invalid"))?;
    Ok(LedgerKey(value))
}

pub(crate) fn read_bounded(file: &File, limit: u64) -> Result<Vec<u8>, RoutineError> {
    let metadata = file
        .metadata()
        .map_err(|_| error("routine-production-authority-entry-stat-failed"))?;
    if metadata.len() > limit {
        return Err(error("routine-production-authority-entry-oversize"));
    }
    let mut cloned = file
        .try_clone()
        .map_err(|_| error("routine-production-authority-entry-read-failed"))?;
    cloned
        .seek(SeekFrom::Start(0))
        .map_err(|_| error("routine-production-authority-entry-read-failed"))?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    cloned
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|_| error("routine-production-authority-entry-read-failed"))?;
    if bytes.len() as u64 > limit || bytes.len() as u64 != metadata.len() {
        return Err(error("routine-production-authority-entry-read-raced"));
    }
    Ok(bytes)
}

pub(crate) fn now_tick() -> Result<u64, RoutineError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| error("routine-production-trusted-time-unavailable"))
}

pub(crate) fn temporary_name() -> Result<String, RoutineError> {
    let mut nonce = [0u8; 16];
    fill(&mut nonce).map_err(|_| error("routine-production-authority-random-unavailable"))?;
    Ok(format!(
        ".routine-authority-state.tmp.{}",
        nonce
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    ))
}

pub(crate) fn validate_name(name: &str) -> Result<(), RoutineError> {
    if matches!(name, KEY_NAME | LOCK_NAME | STATE_NAME)
        || name
            .strip_prefix(".routine-authority-state.tmp.")
            .is_some_and(|suffix| {
                suffix.len() == 32 && suffix.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
    {
        Ok(())
    } else {
        Err(error("routine-production-authority-name-invalid"))
    }
}

pub(crate) fn rename_relative(directory: &File, from: &str, to: &str) -> Result<(), RoutineError> {
    let from =
        CString::new(from).map_err(|_| error("routine-production-authority-name-invalid"))?;
    let to = CString::new(to).map_err(|_| error("routine-production-authority-name-invalid"))?;
    if unsafe {
        libc::renameat(
            directory.as_raw_fd(),
            from.as_ptr(),
            directory.as_raw_fd(),
            to.as_ptr(),
        )
    } != 0
    {
        return Err(error("routine-production-authority-state-publish-failed"));
    }
    Ok(())
}

pub(crate) fn unlink_relative(directory: &File, name: &str) -> Result<(), RoutineError> {
    let name =
        CString::new(name).map_err(|_| error("routine-production-authority-name-invalid"))?;
    if unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), 0) } != 0 {
        return Err(error("routine-production-authority-temp-cleanup-failed"));
    }
    Ok(())
}

pub(crate) fn root_identity(metadata: &fs::Metadata) -> RootIdentity {
    RootIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
        owner: metadata.uid(),
        mode: metadata.mode(),
    }
}

pub(crate) fn file_identity(metadata: &fs::Metadata) -> FileIdentity {
    FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
        owner: metadata.uid(),
        mode: metadata.mode(),
        links: metadata.nlink(),
        length: metadata.len(),
        changed_seconds: metadata.ctime(),
        changed_nanos: metadata.ctime_nsec(),
    }
}

pub(crate) fn stat_identity(stat: &libc::stat) -> FileIdentity {
    FileIdentity {
        device: stat.st_dev as u64,
        inode: stat.st_ino,
        owner: stat.st_uid,
        mode: u32::from(stat.st_mode),
        links: u64::from(stat.st_nlink),
        length: stat.st_size as u64,
        changed_seconds: stat.st_ctime,
        changed_nanos: stat.st_ctime_nsec,
    }
}

pub(crate) fn descriptor_path(file: &File) -> Result<PathBuf, RoutineError> {
    let mut bytes = vec![0u8; libc::PATH_MAX as usize];
    if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETPATH, bytes.as_mut_ptr()) } != 0 {
        return Err(error(
            "routine-production-authority-root-descriptor-path-failed",
        ));
    }
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    Ok(PathBuf::from(OsStr::from_bytes(&bytes[..end])))
}
