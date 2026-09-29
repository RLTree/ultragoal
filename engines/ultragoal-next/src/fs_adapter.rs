//! Bounded macOS descriptor-relative observation. Bytes remain untrusted until the caller admits them.

use crate::os;
use std::fs::File;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

pub(crate) const HARD_MAX_PATH_BYTES: usize = 4096;
const HARD_MAX_EXCLUSIONS: usize = 256;
const HARD_MAX_BASENAME_BYTES: usize = 255;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Limits {
    pub max_entries: usize,
    pub max_depth: usize,
    pub max_path_bytes: usize,
    pub max_file_bytes: usize,
    pub max_total_bytes: usize,
}

impl Limits {
    fn bounded(self) -> Self {
        Self {
            max_entries: self.max_entries,
            max_depth: self.max_depth,
            max_path_bytes: self.max_path_bytes.min(HARD_MAX_PATH_BYTES),
            max_file_bytes: self.max_file_bytes,
            max_total_bytes: self.max_total_bytes,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    File,
    Directory,
    Symlink,
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Member {
    pub path: Vec<u8>,
    pub kind: Kind,
    pub identity: Identity,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Problem {
    InvalidPath,
    Limit,
    EntryBound,
    ResourcePressure,
    Excluded,
    Unavailable(i32),
    Unsupported,
    Unstable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Issue {
    pub path: Vec<u8>,
    pub problem: Problem,
}

#[derive(Debug)]
pub struct Listing {
    pub root_identity: Option<Identity>,
    pub excluded_basenames: Vec<Vec<u8>>,
    pub members: Vec<Member>,
    pub issues: Vec<Issue>,
    pub complete: bool,
    scanned: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Identity {
    pub device: u64,
    pub inode: u64,
    pub size: u64,
    pub mode: u32,
    pub links: u64,
    pub modified: (i64, i64),
    pub changed: (i64, i64),
}

impl Identity {
    fn of(file: &File) -> io::Result<Self> {
        let data = file.metadata()?;
        Ok(Self {
            device: data.dev(),
            inode: data.ino(),
            size: data.size(),
            mode: data.mode(),
            links: data.nlink(),
            modified: (data.mtime(), data.mtime_nsec()),
            changed: (data.ctime(), data.ctime_nsec()),
        })
    }

    fn from_stat(data: &libc::stat) -> Self {
        Self {
            device: data.st_dev as u64,
            inode: data.st_ino,
            size: data.st_size as u64,
            mode: data.st_mode as u32,
            links: data.st_nlink as u64,
            modified: (data.st_mtime, data.st_mtime_nsec),
            changed: (data.st_ctime, data.st_ctime_nsec),
        }
    }
}

#[derive(Debug)]
pub struct Capture {
    pub path: Vec<u8>,
    pub bytes: Option<Vec<u8>>,
    pub identity: Option<Identity>,
    pub problem: Option<Problem>,
}

#[derive(Debug)]
pub struct Snapshot {
    pub listing: Listing,
    pub captures: Vec<Capture>,
}

pub fn snapshot(
    root: &Path,
    selected: Option<&[Vec<u8>]>,
    limits: &Limits,
) -> io::Result<Snapshot> {
    Ok(Root::open(root)?.observe(selected, limits))
}

pub fn snapshot_excluding(
    root: &Path,
    selected: Option<&[Vec<u8>]>,
    limits: &Limits,
    excluded_basenames: &[Vec<u8>],
) -> io::Result<Snapshot> {
    Ok(Root::open(root)?.observe_excluding(selected, limits, excluded_basenames))
}

fn valid_exclusions(names: &[Vec<u8>]) -> bool {
    names.len() <= HARD_MAX_EXCLUSIONS
        && names.iter().all(|name| {
            !name.is_empty()
                && name.len() <= HARD_MAX_BASENAME_BYTES
                && name != b"."
                && name != b".."
                && !name.contains(&b'/')
                && !name.contains(&0)
        })
}

impl Capture {
    fn reject(path: &[u8], problem: Problem) -> Self {
        Self {
            path: path[..path.len().min(HARD_MAX_PATH_BYTES)].to_vec(),
            bytes: None,
            identity: None,
            problem: Some(problem),
        }
    }
}

pub struct Root {
    directory: File,
    path: PathBuf,
}

impl Root {
    /// Identity of the held root descriptor, for an explicit scoped proposal.
    pub fn identity(&self) -> io::Result<Identity> { Identity::of(&self.directory) }

    /// This is path/object binding, not a full-tree or authorization claim.
    pub fn binding_current(&self, expected: Identity) -> bool {
        let same = |id: Identity| id.device == expected.device && id.inode == expected.inode;
        Identity::of(&self.directory).is_ok_and(same)
            && os::open_root(&self.path).and_then(|f| Identity::of(&f)).is_ok_and(same)
    }

    pub fn open(path: &Path) -> io::Result<Self> {
        let directory = os::open_root(path)?;
        Ok(Self {
            directory,
            path: path.to_path_buf(),
        })
    }
}

fn code(error: io::Error) -> i32 {
    error.raw_os_error().unwrap_or(libc::EIO)
}

mod capture;
mod stream;
pub use stream::{StreamError, StreamedSource};
mod membership;
pub use membership::{MembershipCheckpoint, MembershipFallback, MembershipObservation};
mod observe;
mod scan;
#[cfg(test)]
use capture::Phase;

#[cfg(test)]
mod tests;
