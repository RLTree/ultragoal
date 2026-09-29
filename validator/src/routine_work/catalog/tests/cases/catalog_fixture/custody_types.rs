#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum EntryKind {
    Directory,
    File,
    Symlink,
    Special,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) struct EntryIdentity {
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) kind: EntryKind,
}
