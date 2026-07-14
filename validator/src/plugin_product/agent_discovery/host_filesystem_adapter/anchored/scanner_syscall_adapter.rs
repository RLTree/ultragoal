use std::ffi::CStr;
use std::os::fd::RawFd;

pub(super) struct DirectoryStream {
    raw: *mut libc::DIR,
    closed: bool,
}

pub(super) enum ScannerSyscallRequest<'a> {
    Open {
        directory_fd: RawFd,
    },
    Read {
        stream: &'a mut DirectoryStream,
        injected_error: Option<i32>,
    },
    Close {
        stream: &'a mut DirectoryStream,
    },
}

pub(super) enum ScannerSyscallResponse {
    Stream(DirectoryStream),
    Entry(Option<Vec<u8>>),
    Closed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ScannerSyscallError {
    OpenRejected,
    ReadRejected,
    CloseRejected,
    AlreadyClosed,
}

#[cfg(test)]
pub(super) const INTERRUPTED_ERROR: i32 = libc::EINTR;
#[cfg(test)]
pub(super) const IO_ERROR: i32 = libc::EIO;

pub(super) fn open(directory_fd: RawFd) -> Result<DirectoryStream, ScannerSyscallError> {
    match execute(ScannerSyscallRequest::Open { directory_fd })? {
        ScannerSyscallResponse::Stream(stream) => Ok(stream),
        _ => Err(ScannerSyscallError::OpenRejected),
    }
}

pub(super) fn read(
    stream: &mut DirectoryStream,
    injected_error: Option<i32>,
) -> Result<Option<Vec<u8>>, ScannerSyscallError> {
    match execute(ScannerSyscallRequest::Read {
        stream,
        injected_error,
    })? {
        ScannerSyscallResponse::Entry(entry) => Ok(entry),
        _ => Err(ScannerSyscallError::ReadRejected),
    }
}

pub(super) fn close(stream: &mut DirectoryStream) -> Result<(), ScannerSyscallError> {
    match execute(ScannerSyscallRequest::Close { stream })? {
        ScannerSyscallResponse::Closed => Ok(()),
        _ => Err(ScannerSyscallError::CloseRejected),
    }
}

fn execute(
    request: ScannerSyscallRequest<'_>,
) -> Result<ScannerSyscallResponse, ScannerSyscallError> {
    match request {
        ScannerSyscallRequest::Open { directory_fd } => {
            let current = c".";
            let duplicate = unsafe {
                libc::openat(
                    directory_fd,
                    current.as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                )
            };
            if duplicate < 0 {
                return Err(ScannerSyscallError::OpenRejected);
            }
            let raw = unsafe { libc::fdopendir(duplicate) };
            if raw.is_null() {
                unsafe { libc::close(duplicate) };
                return Err(ScannerSyscallError::OpenRejected);
            }
            Ok(ScannerSyscallResponse::Stream(DirectoryStream {
                raw,
                closed: false,
            }))
        }
        ScannerSyscallRequest::Read {
            stream,
            injected_error,
        } => {
            if stream.closed {
                return Err(ScannerSyscallError::AlreadyClosed);
            }
            set_errno(0)?;
            let entry = if let Some(error) = injected_error {
                set_errno(error)?;
                std::ptr::null_mut()
            } else {
                unsafe { libc::readdir(stream.raw) }
            };
            if entry.is_null() {
                return match get_errno()? {
                    0 => Ok(ScannerSyscallResponse::Entry(None)),
                    _ => Err(ScannerSyscallError::ReadRejected),
                };
            }
            let bytes = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }
                .to_bytes()
                .to_vec();
            Ok(ScannerSyscallResponse::Entry(Some(bytes)))
        }
        ScannerSyscallRequest::Close { stream } => {
            if stream.closed {
                return Err(ScannerSyscallError::AlreadyClosed);
            }
            stream.closed = true;
            if unsafe { libc::closedir(stream.raw) } != 0 {
                Err(ScannerSyscallError::CloseRejected)
            } else {
                Ok(ScannerSyscallResponse::Closed)
            }
        }
    }
}

impl Drop for DirectoryStream {
    fn drop(&mut self) {
        if !self.closed {
            self.closed = true;
            unsafe { libc::closedir(self.raw) };
        }
    }
}

fn set_errno(value: i32) -> Result<(), ScannerSyscallError> {
    let pointer = errno_pointer();
    if pointer.is_null() {
        return Err(ScannerSyscallError::ReadRejected);
    }
    unsafe { *pointer = value };
    Ok(())
}

fn get_errno() -> Result<i32, ScannerSyscallError> {
    let pointer = errno_pointer();
    if pointer.is_null() {
        return Err(ScannerSyscallError::ReadRejected);
    }
    Ok(unsafe { *pointer })
}

#[cfg(any(target_vendor = "apple", target_os = "freebsd"))]
fn errno_pointer() -> *mut libc::c_int {
    unsafe { libc::__error() }
}

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "emscripten",
    target_os = "hurd",
    target_os = "redox"
))]
fn errno_pointer() -> *mut libc::c_int {
    unsafe { libc::__errno_location() }
}

#[cfg(any(
    target_os = "android",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "cygwin",
    target_os = "nuttx"
))]
fn errno_pointer() -> *mut libc::c_int {
    unsafe { libc::__errno() }
}

#[cfg(any(target_os = "illumos", target_os = "solaris"))]
fn errno_pointer() -> *mut libc::c_int {
    unsafe { libc::___errno() }
}

#[cfg(not(any(
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "emscripten",
    target_os = "hurd",
    target_os = "redox",
    target_os = "android",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "cygwin",
    target_os = "nuttx",
    target_os = "illumos",
    target_os = "solaris"
)))]
fn errno_pointer() -> *mut libc::c_int {
    std::ptr::null_mut()
}
