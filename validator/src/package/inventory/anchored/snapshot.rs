use std::fs::Metadata;
use std::os::unix::fs::MetadataExt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EntryKind {
    Regular { single_link: bool },
    Directory,
    Symlink,
    Special,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Snapshot {
    device: u64,
    inode: u64,
    mode: u32,
    links: u64,
    size: i64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

impl Snapshot {
    pub(super) fn from_metadata(metadata: &Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            mode: metadata.mode(),
            links: metadata.nlink(),
            size: i64::try_from(metadata.size()).unwrap_or(i64::MAX),
            modified_seconds: metadata.mtime(),
            modified_nanoseconds: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanoseconds: metadata.ctime_nsec(),
        }
    }

    pub(super) fn from_stat(stat: &libc::stat) -> Self {
        Self {
            device: stat.st_dev as u64,
            inode: stat.st_ino,
            mode: stat.st_mode as u32,
            links: stat.st_nlink as u64,
            size: stat.st_size,
            modified_seconds: stat.st_mtime,
            modified_nanoseconds: stat.st_mtime_nsec,
            changed_seconds: stat.st_ctime,
            changed_nanoseconds: stat.st_ctime_nsec,
        }
    }

    pub(super) fn is_directory(self) -> bool {
        self.mode & u32::from(libc::S_IFMT) == u32::from(libc::S_IFDIR)
    }

    pub(super) fn is_regular(self) -> bool {
        self.mode & u32::from(libc::S_IFMT) == u32::from(libc::S_IFREG)
    }

    pub(super) fn entry_kind(self) -> EntryKind {
        let kind = self.mode & u32::from(libc::S_IFMT);
        if kind == u32::from(libc::S_IFREG) {
            EntryKind::Regular {
                single_link: self.links == 1,
            }
        } else if kind == u32::from(libc::S_IFDIR) {
            EntryKind::Directory
        } else if kind == u32::from(libc::S_IFLNK) {
            EntryKind::Symlink
        } else {
            EntryKind::Special
        }
    }

    pub(super) fn same_identity(self, other: Self) -> bool {
        self.device == other.device && self.inode == other.inode && self.mode == other.mode
    }

    pub(super) fn links(self) -> u64 {
        self.links
    }

    pub(super) fn size(self) -> i64 {
        self.size
    }

    pub(super) fn unix_mode(self) -> u32 {
        self.mode & 0o777
    }
}
