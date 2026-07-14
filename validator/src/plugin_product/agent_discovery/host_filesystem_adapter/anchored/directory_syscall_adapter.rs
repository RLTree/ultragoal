use std::ffi::CStr;
use std::os::fd::RawFd;

#[derive(Clone, Copy)]
enum DirectoryOpenKind {
    Directory,
    RegularFile,
}

pub(super) struct DirectorySyscallRequest<'a> {
    directory_fd: RawFd,
    name: &'a CStr,
    kind: DirectoryOpenKind,
}

pub(super) struct DirectorySyscallResponse {
    descriptor: RawFd,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum DirectorySyscallError {
    OpenRejected,
}

pub(super) fn open_directory(
    directory_fd: RawFd,
    name: &CStr,
) -> Result<RawFd, DirectorySyscallError> {
    execute(DirectorySyscallRequest {
        directory_fd,
        name,
        kind: DirectoryOpenKind::Directory,
    })
    .map(|response| response.descriptor)
}

pub(super) fn open_regular_file(
    directory_fd: RawFd,
    name: &CStr,
) -> Result<RawFd, DirectorySyscallError> {
    execute(DirectorySyscallRequest {
        directory_fd,
        name,
        kind: DirectoryOpenKind::RegularFile,
    })
    .map(|response| response.descriptor)
}

fn execute(
    request: DirectorySyscallRequest<'_>,
) -> Result<DirectorySyscallResponse, DirectorySyscallError> {
    let flags = match request.kind {
        DirectoryOpenKind::Directory => {
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC
        }
        DirectoryOpenKind::RegularFile => {
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK
        }
    };
    let descriptor = unsafe { libc::openat(request.directory_fd, request.name.as_ptr(), flags) };
    if descriptor < 0 {
        Err(DirectorySyscallError::OpenRejected)
    } else {
        Ok(DirectorySyscallResponse { descriptor })
    }
}
