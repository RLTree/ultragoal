use std::ffi::CStr;
use std::os::fd::RawFd;

pub(super) enum IdentitySyscallRequest<'a> {
    OpenRoot { path: &'a CStr },
    InspectRegular { directory_fd: RawFd, name: &'a CStr },
}

pub(super) enum IdentitySyscallResponse {
    Descriptor(RawFd),
    RegularIdentity { mode: u32, links: u64 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum IdentitySyscallError {
    OpenRejected,
    MetadataRejected,
}

pub(super) fn open_root(path: &CStr) -> Result<RawFd, IdentitySyscallError> {
    match execute(IdentitySyscallRequest::OpenRoot { path })? {
        IdentitySyscallResponse::Descriptor(descriptor) => Ok(descriptor),
        _ => Err(IdentitySyscallError::OpenRejected),
    }
}

pub(super) fn require_regular_single_link_at(
    directory_fd: RawFd,
    name: &CStr,
) -> Result<(), IdentitySyscallError> {
    match execute(IdentitySyscallRequest::InspectRegular { directory_fd, name })? {
        IdentitySyscallResponse::RegularIdentity { mode, links }
            if mode & u32::from(libc::S_IFMT) == u32::from(libc::S_IFREG) && links == 1 =>
        {
            Ok(())
        }
        _ => Err(IdentitySyscallError::MetadataRejected),
    }
}

fn execute(
    request: IdentitySyscallRequest<'_>,
) -> Result<IdentitySyscallResponse, IdentitySyscallError> {
    match request {
        IdentitySyscallRequest::OpenRoot { path } => {
            // SAFETY: `path` is a live, NUL-terminated C string for this call.
            let descriptor = unsafe {
                libc::open(
                    path.as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                )
            };
            if descriptor < 0 {
                Err(IdentitySyscallError::OpenRejected)
            } else {
                Ok(IdentitySyscallResponse::Descriptor(descriptor))
            }
        }
        IdentitySyscallRequest::InspectRegular { directory_fd, name } => {
            let mut metadata = std::mem::MaybeUninit::<libc::stat>::uninit();
            // SAFETY: `name` is NUL-terminated and `metadata` provides writable `stat` storage.
            let result = unsafe {
                libc::fstatat(
                    directory_fd,
                    name.as_ptr(),
                    metadata.as_mut_ptr(),
                    libc::AT_SYMLINK_NOFOLLOW,
                )
            };
            if result != 0 {
                return Err(IdentitySyscallError::MetadataRejected);
            }
            // SAFETY: successful `fstatat` initialized the entire `stat` value.
            let metadata = unsafe { metadata.assume_init() };
            Ok(IdentitySyscallResponse::RegularIdentity {
                mode: metadata.st_mode as u32,
                links: metadata.st_nlink as u64,
            })
        }
    }
}
