use super::super::{EvaluationError, MAX_INPUT_BYTES, digest, safe_relative_path};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ProductionInputIdentity {
    device: u64,
    inode: u64,
    mode: u32,
    links: u64,
    user: u32,
    length: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

pub(super) struct ProtectedProductionInput {
    relative_path: String,
    digest_sha256: String,
    identity: ProductionInputIdentity,
    descriptor: std::fs::File,
}

impl ProtectedProductionInput {
    pub(super) fn capture_observed(
        root: &std::fs::File,
        path: &str,
    ) -> Result<Self, EvaluationError> {
        Self::capture_inner(root, path, None, None)
    }

    pub(super) fn capture(
        root: &std::fs::File,
        path: &str,
        digest: &str,
        length: Option<u64>,
    ) -> Result<Self, EvaluationError> {
        Self::capture_inner(root, path, Some(digest), length)
    }

    fn capture_inner(
        root: &std::fs::File,
        path: &str,
        expected_digest: Option<&str>,
        expected_length: Option<u64>,
    ) -> Result<Self, EvaluationError> {
        let descriptor = open_production_input(root, path)?;
        let identity = production_identity(&descriptor)?;
        if identity.mode & libc::S_IFMT as u32 != libc::S_IFREG as u32
            || identity.links != 1
            || identity.user != unsafe { libc::geteuid() }
            || identity.length == 0
            || identity.length > MAX_INPUT_BYTES
            || expected_length.is_some_and(|value| value != identity.length)
        {
            return Err(EvaluationError::new("evaluation-production-input-unsafe"));
        }
        let bytes = read_production_input(&descriptor, identity.length)?;
        let observed_digest = digest(&bytes);
        if expected_digest.is_some_and(|value| value != observed_digest) {
            return Err(EvaluationError::new(
                "evaluation-production-input-binding-mismatch",
            ));
        }
        Ok(Self {
            relative_path: path.to_owned(),
            digest_sha256: observed_digest,
            identity,
            descriptor,
        })
    }

    pub(super) fn digest_sha256(&self) -> &str {
        &self.digest_sha256
    }

    pub(super) fn read_authenticated(&self) -> Result<Vec<u8>, EvaluationError> {
        if production_identity(&self.descriptor)? != self.identity {
            return Err(EvaluationError::new("evaluation-production-input-changed"));
        }
        let bytes = read_production_input(&self.descriptor, self.identity.length)?;
        if digest(&bytes) != self.digest_sha256 {
            return Err(EvaluationError::new("evaluation-production-input-changed"));
        }
        Ok(bytes)
    }

    pub(super) fn revalidate(&self, root: &std::fs::File) -> Result<(), EvaluationError> {
        let named = open_production_input(root, &self.relative_path)?;
        if production_identity(&named)? != self.identity {
            return Err(EvaluationError::new("evaluation-production-input-changed"));
        }
        self.read_authenticated().map(|_| ())
    }
}

pub(super) fn open_production_root(
    path: &std::path::Path,
) -> Result<std::fs::File, EvaluationError> {
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::io::FromRawFd;
    let path = std::ffi::CString::new(path.as_os_str().as_bytes())
        .map_err(|_| EvaluationError::new("evaluation-input-root-invalid"))?;
    let fd = unsafe {
        libc::open(
            path.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(EvaluationError::new("evaluation-input-root-open-failed"));
    }
    Ok(unsafe { std::fs::File::from_raw_fd(fd) })
}

fn open_production_input(
    root: &std::fs::File,
    relative_path: &str,
) -> Result<std::fs::File, EvaluationError> {
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::unix::ffi::OsStrExt;
    if !safe_relative_path(relative_path) {
        return Err(EvaluationError::new(
            "evaluation-production-input-path-invalid",
        ));
    }
    let components = std::path::Path::new(relative_path)
        .components()
        .map(|component| match component {
            std::path::Component::Normal(value) => value,
            _ => unreachable!(),
        })
        .collect::<Vec<_>>();
    let mut directory = root
        .try_clone()
        .map_err(|_| EvaluationError::new("evaluation-input-root-clone-failed"))?;
    for (index, component) in components.iter().enumerate() {
        let component = std::ffi::CString::new(component.as_bytes())
            .map_err(|_| EvaluationError::new("evaluation-production-input-path-invalid"))?;
        let last = index + 1 == components.len();
        let flags = libc::O_RDONLY
            | libc::O_NOFOLLOW
            | libc::O_CLOEXEC
            | if last { 0 } else { libc::O_DIRECTORY };
        let fd = unsafe { libc::openat(directory.as_raw_fd(), component.as_ptr(), flags) };
        if fd < 0 {
            return Err(EvaluationError::new(
                "evaluation-production-input-open-failed",
            ));
        }
        let opened = unsafe { std::fs::File::from_raw_fd(fd) };
        if last {
            return Ok(opened);
        }
        directory = opened;
    }
    Err(EvaluationError::new(
        "evaluation-production-input-path-invalid",
    ))
}

pub(super) fn production_identity(
    file: &std::fs::File,
) -> Result<ProductionInputIdentity, EvaluationError> {
    use std::os::unix::fs::MetadataExt;
    let metadata = file
        .metadata()
        .map_err(|_| EvaluationError::new("evaluation-production-input-stat-failed"))?;
    Ok(ProductionInputIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
        mode: metadata.mode(),
        links: metadata.nlink(),
        user: metadata.uid(),
        length: metadata.size(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanoseconds: metadata.ctime_nsec(),
    })
}

fn read_production_input(file: &std::fs::File, length: u64) -> Result<Vec<u8>, EvaluationError> {
    use std::os::unix::fs::FileExt;
    let mut bytes = vec![0_u8; length as usize];
    let mut offset = 0;
    while offset < bytes.len() {
        let count = file
            .read_at(&mut bytes[offset..], offset as u64)
            .map_err(|_| EvaluationError::new("evaluation-production-input-read-failed"))?;
        if count == 0 {
            return Err(EvaluationError::new(
                "evaluation-production-input-read-truncated",
            ));
        }
        offset += count;
    }
    Ok(bytes)
}
