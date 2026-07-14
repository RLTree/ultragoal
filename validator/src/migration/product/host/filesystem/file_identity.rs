#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct FileIdentity {
    pub(super) device: u64,
    pub(super) inode: u64,
    pub(super) uid: u32,
    pub(super) mode: u32,
    pub(super) links: u64,
    pub(super) size: u64,
    pub(super) kind: u32,
}

impl FileIdentity {
    pub(super) fn safe_regular(self, exact_owner_only: bool, max_bytes: u64) -> bool {
        self.kind == libc::S_IFREG as u32
            && self.uid == unsafe { libc::geteuid() }
            && self.links == 1
            && self.size <= max_bytes
            && self.mode & 0o022 == 0
            && (!exact_owner_only || self.mode == 0o600)
    }

    fn same_directory(self, other: Self) -> bool {
        self.device == other.device
            && self.inode == other.inode
            && self.uid == other.uid
            && self.mode == other.mode
            && self.kind == other.kind
            && self.kind == libc::S_IFDIR as u32
    }
}

#[derive(Debug)]
pub(super) struct AnchoredDirectory {
    path: PathBuf,
    file: File,
    identity: FileIdentity,
    owner_only: bool,
}
