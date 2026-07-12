use super::filesystem::RootAnchor;
use super::util::{bytes_hex, is_prohibited_component, os_bytes};
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
struct Stamp {
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
        let mut entries = BTreeMap::new();
        for item in WalkDir::new(root)
            .follow_links(false)
            .max_depth(MAX_TREE_DEPTH)
            .sort_by_file_name()
            .into_iter()
            .filter_entry(|item| item.depth() == 0 || !is_prohibited_component(item.file_name()))
        {
            let item = item.map_err(|_| "worktree witness traversal failed".to_owned())?;
            if item.depth() == MAX_TREE_DEPTH && item.file_type().is_dir() {
                return Err("worktree witness exceeds depth bound".to_owned());
            }
            if entries.len() >= MAX_TREE_ENTRIES {
                return Err("worktree witness exceeds entry bound".to_owned());
            }
            let relative = item
                .path()
                .strip_prefix(root)
                .map_err(|_| "worktree witness path escaped root".to_owned())?;
            let metadata = fs::symlink_metadata(item.path())
                .map_err(|_| "worktree witness metadata failed".to_owned())?;
            entries.insert(
                bytes_hex(os_bytes(relative.as_os_str())),
                Stamp::from(&metadata),
            );
        }
        Ok(entries)
    }

    #[cfg(not(unix))]
    fn capture(_root: &Path) -> Result<(), String> {
        Err("read-effect mutation witness requires Unix".to_owned())
    }
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
