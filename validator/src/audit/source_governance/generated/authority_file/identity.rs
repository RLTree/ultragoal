use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum NodeKind {
    Directory,
    RegularFile,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct NodeIdentity {
    pub(super) device: u64,
    pub(super) inode: u64,
    pub(super) kind: NodeKind,
    pub(super) mode: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::audit::source_governance) struct AuthorityFileBaseline {
    relative: PathBuf,
    root: NodeIdentity,
    ancestors: Vec<NodeIdentity>,
    leaf: NodeIdentity,
    digest: String,
    length: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BaselineChange {
    Root,
    Ancestor,
    Leaf,
    Content,
}

impl AuthorityFileBaseline {
    pub(super) fn new(
        relative: PathBuf,
        root: NodeIdentity,
        ancestors: Vec<NodeIdentity>,
        leaf: NodeIdentity,
        bytes: &[u8],
    ) -> Self {
        Self {
            relative,
            root,
            ancestors,
            leaf,
            digest: crate::digest::bytes(bytes),
            length: bytes.len() as u64,
        }
    }

    pub(super) fn relative(&self) -> &Path {
        &self.relative
    }

    pub(super) fn change(&self, current: &Self) -> Option<BaselineChange> {
        if self.relative != current.relative || self.root != current.root {
            Some(BaselineChange::Root)
        } else if self.ancestors != current.ancestors {
            Some(BaselineChange::Ancestor)
        } else if self.leaf != current.leaf {
            Some(BaselineChange::Leaf)
        } else if self.digest != current.digest || self.length != current.length {
            Some(BaselineChange::Content)
        } else {
            None
        }
    }
}
