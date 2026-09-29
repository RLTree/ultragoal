use std::ffi::{CStr, CString};
use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

#[cfg(test)]
thread_local! {
    static READDIR_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn readdir_calls() -> usize {
    READDIR_CALLS.with(std::cell::Cell::get)
}

pub(crate) fn open_root(path: &Path) -> io::Result<File> {
    let bytes = path.as_os_str().as_bytes();
    if bytes.first() != Some(&b'/')
        || bytes.contains(&0)
        || bytes.len() > crate::fs_adapter::HARD_MAX_PATH_BYTES
        || (bytes.len() > 1
            && bytes[1..]
                .split(|byte| *byte == b'/')
                .any(|part| part.is_empty() || part == b"." || part == b".."))
    {
        return Err(io::Error::from(io::ErrorKind::InvalidInput));
    }
    // One resolution that refuses a symbolic link in every component.
    let path = CString::new(bytes).unwrap();
    owned(unsafe {
        libc::open(
            path.as_ptr(),
            (dir_flags() & !libc::O_NOFOLLOW) | libc::O_NOFOLLOW_ANY,
        )
    })
}

pub(crate) fn open_dir(parent: &File, name: &CStr) -> io::Result<File> {
    owned(unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), dir_flags()) })
}

pub(crate) fn open_regular(parent: &File, name: &CStr) -> io::Result<File> {
    owned(unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        )
    })
}

fn dir_flags() -> libc::c_int {
    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK
}

fn owned(fd: libc::c_int) -> io::Result<File> {
    if fd < 0 {
        Err(io::Error::last_os_error())
    } else {
        // SAFETY: successful open/openat returned one unique owned descriptor.
        Ok(unsafe { File::from_raw_fd(fd) })
    }
}

pub(crate) fn entry_stat(parent: &File, name: &CStr) -> io::Result<libc::stat> {
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    let result = unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        // SAFETY: successful fstatat initialized the struct.
        Ok(unsafe { stat.assume_init() })
    }
}

pub(crate) struct DirStream(*mut libc::DIR);

impl DirStream {
    pub(crate) fn new(directory: &File) -> io::Result<Self> {
        let dot = CString::new(".").unwrap();
        // A fresh open file description is required: dup shares the directory offset.
        let fd = unsafe { libc::openat(directory.as_raw_fd(), dot.as_ptr(), dir_flags()) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let stream = unsafe { libc::fdopendir(fd) };
        if stream.is_null() {
            let error = io::Error::last_os_error();
            unsafe { libc::close(fd) };
            Err(error)
        } else {
            Ok(Self(stream))
        }
    }

    pub(crate) fn next_name(&mut self) -> io::Result<Option<Vec<u8>>> {
        loop {
            #[cfg(test)]
            READDIR_CALLS.with(|calls| calls.set(calls.get() + 1));
            unsafe { *libc::__error() = 0 };
            let entry = unsafe { libc::readdir(self.0) };
            if entry.is_null() {
                let error = io::Error::last_os_error();
                return if error.raw_os_error() == Some(0) {
                    Ok(None)
                } else {
                    Err(error)
                };
            }
            let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
            if name != b"." && name != b".." {
                return Ok(Some(name.to_vec()));
            }
        }
    }

    pub(crate) fn close(&mut self) -> io::Result<()> {
        if self.0.is_null() {
            return Ok(());
        }
        let stream = std::mem::replace(&mut self.0, std::ptr::null_mut());
        if unsafe { libc::closedir(stream) } < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

impl Drop for DirStream {
    fn drop(&mut self) {
        let _ = self.close();
    }
}
