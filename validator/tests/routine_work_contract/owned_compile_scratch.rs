use std::ffi::{CStr, CString};
use std::fs::{self, File};
use std::io::{self, Write};
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_SCRATCH: AtomicU64 = AtomicU64::new(0);
pub(crate) const FAILURE_MARKER: &[u8] = b"compile scratch removed after induced failure\n";
pub(crate) const SUBSTITUTION_MARKER: &[u8] =
    b"renamed compile scratch emptied after path substitution\n";
pub(crate) const SUBSTITUTION_MARKER_NAME: &str = "RETAINED-SUBSTITUTION.txt";

pub(crate) struct OwnedCompileScratch {
    parent: File,
    directory: File,
    name: CString,
    path: PathBuf,
    failure_marker: PathBuf,
    device: u64,
    inode: u64,
}

impl OwnedCompileScratch {
    pub(crate) fn claim(label: &str) -> Self {
        let root = configured_root("CODEX_WORKTREE_SCRATCH");
        let _ = configured_root("CODEX_WORKTREE_TMP");
        let parent = File::open(&root).expect("configured scratch opens");
        for _ in 0..64 {
            let nonce = NEXT_SCRATCH.fetch_add(1, Ordering::Relaxed);
            let name = CString::new(format!("{label}-{}-{nonce}", std::process::id())).unwrap();
            let created = unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) };
            if created != 0 {
                let error = io::Error::last_os_error();
                if error.kind() == io::ErrorKind::AlreadyExists {
                    continue;
                }
                panic!("compile scratch claim failed: {error}");
            }
            let directory = open_directory_at(parent.as_raw_fd(), &name).unwrap();
            write_new_file(
                directory.as_raw_fd(),
                "OWNERSHIP.txt",
                b"descriptor-owned compile scratch\n",
            )
            .unwrap();
            let metadata = directory.metadata().unwrap();
            let path = root.join(name.to_str().unwrap());
            let failure_marker = root.join(format!("{}.failure.txt", name.to_str().unwrap()));
            return Self {
                parent,
                directory,
                name,
                path,
                failure_marker,
                device: metadata.dev(),
                inode: metadata.ino(),
            };
        }
        panic!("compile scratch claim collisions exhausted")
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn failure_marker(&self) -> &Path {
        &self.failure_marker
    }
}

impl Drop for OwnedCompileScratch {
    fn drop(&mut self) {
        let panicking = std::thread::panicking();
        let cleared = clear_directory(self.directory.as_raw_fd());
        if cleared.is_ok() && self.original_entry_is_empty() {
            let removed = unsafe {
                libc::unlinkat(
                    self.parent.as_raw_fd(),
                    self.name.as_ptr(),
                    libc::AT_REMOVEDIR,
                )
            };
            if removed == 0 {
                if panicking {
                    let _ = write_new_file_at_path(
                        self.parent.as_raw_fd(),
                        &self.failure_marker,
                        FAILURE_MARKER,
                    );
                }
                return;
            }
        }
        let _ = write_new_file(
            self.directory.as_raw_fd(),
            SUBSTITUTION_MARKER_NAME,
            SUBSTITUTION_MARKER,
        );
        if !panicking {
            assert!(cleared.is_ok(), "descriptor-owned scratch cleanup failed");
        }
    }
}

impl OwnedCompileScratch {
    fn original_entry_is_empty(&self) -> bool {
        if entry_identity(self.parent.as_raw_fd(), &self.name) != Some((self.device, self.inode)) {
            return false;
        }
        directory_entries(self.directory.as_raw_fd()).is_ok_and(|entries| entries.is_empty())
    }
}

fn configured_root(name: &str) -> PathBuf {
    let value = std::env::var_os(name).unwrap_or_else(|| panic!("{name} is required"));
    let path = fs::canonicalize(PathBuf::from(value))
        .unwrap_or_else(|error| panic!("{name} is unavailable: {error}"));
    assert!(path.is_absolute() && path.is_dir(), "invalid {name}");
    path
}

fn open_directory_at(parent: RawFd, name: &CStr) -> io::Result<File> {
    let fd = unsafe {
        libc::openat(
            parent,
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(unsafe { File::from_raw_fd(fd) })
    }
}

fn entry_identity(parent: RawFd, name: &CStr) -> Option<(u64, u64)> {
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    let result = unsafe {
        libc::fstatat(
            parent,
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    (result == 0).then(|| {
        let stat = unsafe { stat.assume_init() };
        (stat.st_dev as u64, stat.st_ino as u64)
    })
}

fn clear_directory(directory: RawFd) -> io::Result<()> {
    for name in directory_entries(directory)? {
        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        if unsafe {
            libc::fstatat(
                directory,
                name.as_ptr(),
                stat.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        } != 0
        {
            return Err(io::Error::last_os_error());
        }
        let stat = unsafe { stat.assume_init() };
        let directory_entry = stat.st_mode & libc::S_IFMT == libc::S_IFDIR;
        if directory_entry {
            let child = open_directory_at(directory, &name)?;
            clear_directory(child.as_raw_fd())?;
        }
        let flags = if directory_entry {
            libc::AT_REMOVEDIR
        } else {
            0
        };
        if unsafe { libc::unlinkat(directory, name.as_ptr(), flags) } != 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

fn directory_entries(directory: RawFd) -> io::Result<Vec<CString>> {
    let duplicate = unsafe { libc::dup(directory) };
    if duplicate < 0 {
        return Err(io::Error::last_os_error());
    }
    let stream = unsafe { libc::fdopendir(duplicate) };
    if stream.is_null() {
        unsafe { libc::close(duplicate) };
        return Err(io::Error::last_os_error());
    }
    let mut names = Vec::new();
    loop {
        let entry = unsafe { libc::readdir(stream) };
        if entry.is_null() {
            break;
        }
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) };
        if name.to_bytes() != b"." && name.to_bytes() != b".." {
            names.push(CString::new(name.to_bytes()).unwrap());
        }
    }
    if unsafe { libc::closedir(stream) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(names)
}

fn write_new_file(directory: RawFd, name: &str, bytes: &[u8]) -> io::Result<()> {
    let name = CString::new(name).unwrap();
    let fd = unsafe {
        libc::openat(
            directory,
            name.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_CLOEXEC,
            0o600,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    unsafe { File::from_raw_fd(fd) }.write_all(bytes)
}

fn write_new_file_at_path(directory: RawFd, path: &Path, bytes: &[u8]) -> io::Result<()> {
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "marker name missing"))?;
    let name = CString::new(name.as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "marker name invalid"))?;
    write_new_file(directory, name.to_str().unwrap(), bytes)
}
