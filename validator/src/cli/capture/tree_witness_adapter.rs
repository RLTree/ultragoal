use super::filesystem::RootAnchor;
use super::identity_codec::bytes_hex;
use super::path_policy::{is_prohibited_component, os_bytes};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

const MAX_TREE_ENTRIES: usize = 250_000;
const MAX_TREE_DEPTH: usize = 128;

#[cfg(unix)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Stamp {
    device: u64,
    inode: u64,
    links: u64,
    mode: u32,
    length: u64,
    modified_seconds: i64,
    modified_nanos: i64,
    changed_seconds: i64,
    changed_nanos: i64,
}

pub(super) enum TreeWitnessRequest<'a> {
    Capture { root: &'a Path },
}

pub(super) enum TreeWitnessResponse {
    #[cfg(unix)]
    Snapshot(BTreeMap<String, Stamp>),
}

#[derive(Debug)]
pub(super) struct TreeWitnessError {
    detail: &'static str,
}

impl TreeWitnessError {
    fn stable_text(self) -> String {
        self.detail.to_owned()
    }
}

#[cfg(unix)]
impl Stamp {
    fn from(metadata: &fs::Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            links: metadata.nlink(),
            mode: metadata.mode(),
            length: metadata.len(),
            modified_seconds: metadata.mtime(),
            modified_nanos: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanos: metadata.ctime_nsec(),
        }
    }
}

/// Metadata witness used to detect even mutate-and-restore behavior by a
/// structurally read-only subprocess. Access time is deliberately excluded.
pub(super) struct TreeSnapshot {
    #[cfg(unix)]
    entries: BTreeMap<String, Stamp>,
}

impl TreeSnapshot {
    pub fn stable(root: &RootAnchor) -> Result<Self, String> {
        root.validate()?;
        let first = Self::capture(root.path())?;
        let second = Self::capture(root.path())?;
        if first != second {
            return Err("worktree changed while establishing read-effect witness".to_owned());
        }
        root.validate()?;
        Ok(Self {
            #[cfg(unix)]
            entries: second,
        })
    }

    pub fn validate(&self, root: &RootAnchor) -> Result<(), String> {
        let current = Self::stable(root)?;
        #[cfg(unix)]
        if current.entries != self.entries {
            return Err("read-class subprocess mutated worktree metadata".to_owned());
        }
        Ok(())
    }

    #[cfg(unix)]
    fn capture(root: &Path) -> Result<BTreeMap<String, Stamp>, String> {
        match execute(TreeWitnessRequest::Capture { root })
            .map_err(TreeWitnessError::stable_text)?
        {
            TreeWitnessResponse::Snapshot(entries) => Ok(entries),
        }
    }

    #[cfg(not(unix))]
    fn capture(_root: &Path) -> Result<(), String> {
        Err("read-effect mutation witness requires Unix".to_owned())
    }
}

#[cfg(unix)]
pub(super) fn execute(
    request: TreeWitnessRequest<'_>,
) -> Result<TreeWitnessResponse, TreeWitnessError> {
    let TreeWitnessRequest::Capture { root } = request;
    let mut entries = BTreeMap::new();
    for item in WalkDir::new(root)
        .follow_links(false)
        .max_depth(MAX_TREE_DEPTH)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|item| item.depth() == 0 || !is_prohibited_component(item.file_name()))
    {
        let item = item.map_err(|_| TreeWitnessError {
            detail: "worktree witness traversal failed",
        })?;
        if item.depth() == MAX_TREE_DEPTH && item.file_type().is_dir() {
            return Err(TreeWitnessError {
                detail: "worktree witness exceeds depth bound",
            });
        }
        if entries.len() >= MAX_TREE_ENTRIES {
            return Err(TreeWitnessError {
                detail: "worktree witness exceeds entry bound",
            });
        }
        let relative = item
            .path()
            .strip_prefix(root)
            .map_err(|_| TreeWitnessError {
                detail: "worktree witness path escaped root",
            })?;
        let metadata = fs::symlink_metadata(item.path()).map_err(|_| TreeWitnessError {
            detail: "worktree witness metadata failed",
        })?;
        entries.insert(
            bytes_hex(os_bytes(relative.as_os_str())),
            Stamp::from(&metadata),
        );
    }
    Ok(TreeWitnessResponse::Snapshot(entries))
}

#[cfg(unix)]
impl PartialEq for TreeSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.entries == other.entries
    }
}

#[cfg(not(unix))]
impl PartialEq for TreeSnapshot {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}
