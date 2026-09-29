use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::routine_work::{RepoPath, RoutineError, RoutineErrorId};

#[path = "git_visible_tree_descriptor.rs"]
mod descriptor;

const ENTRY_LIMIT: usize = 100_000;
const DEPTH_LIMIT: usize = 64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct GitVisibleWorktreeShape(String);

impl GitVisibleWorktreeShape {
    pub(super) fn capture(root: &Path, ignored: &BTreeSet<RepoPath>) -> Result<Self, RoutineError> {
        #[cfg(not(unix))]
        {
            let _ = (root, ignored);
            return Err(unsupported("worktree-platform-not-supported"));
        }
        #[cfg(unix)]
        {
            let root_directory = descriptor::open_root(root)?;
            let mut hasher = Sha256::new();
            let mut stack = vec![(root_directory, PathBuf::new(), 0_usize)];
            let mut count = 0_usize;
            while let Some((directory, prefix, depth)) = stack.pop() {
                if depth > DEPTH_LIMIT {
                    return Err(limit("worktree-depth-limit-exceeded"));
                }
                let mut entries = descriptor::entries(&directory)?;
                entries.sort_by(|left, right| left.name.cmp(&right.name));
                for entry in entries.into_iter().rev() {
                    let relative = prefix.join(&entry.name);
                    if depth == 0 && relative == Path::new(".git") {
                        continue;
                    }
                    let text = relative
                        .to_str()
                        .ok_or_else(|| unsupported("worktree-path-not-utf8"))?;
                    let parsed = RepoPath::parse(text.to_owned())?;
                    if ignored.iter().any(|ignored| ignored_path(ignored, &parsed)) {
                        continue;
                    }
                    count = count.saturating_add(1);
                    if count > ENTRY_LIMIT {
                        return Err(limit("worktree-entry-limit-exceeded"));
                    }
                    let kind = entry.identity.kind()?;
                    update_identity(&mut hasher, text, kind, &entry.identity);
                    if kind == b'd' {
                        let child = descriptor::open_child_directory(
                            &directory,
                            &entry.name,
                            entry.identity,
                        )?;
                        stack.push((child, relative, depth + 1));
                    }
                }
            }
            Ok(Self(format!("sha256:{:x}", hasher.finalize())))
        }
    }
}

fn ignored_path(prefix: &RepoPath, path: &RepoPath) -> bool {
    path == prefix
        || path
            .as_str()
            .strip_prefix(prefix.as_str())
            .is_some_and(|suffix| suffix.starts_with('/'))
}

#[cfg(unix)]
fn update_identity(
    hasher: &mut Sha256,
    path: &str,
    kind: u8,
    identity: &descriptor::EntryIdentity,
) {
    hasher.update((path.len() as u64).to_be_bytes());
    hasher.update(path.as_bytes());
    hasher.update([kind]);
    hasher.update(identity.length.to_be_bytes());
    hasher.update(identity.device.to_be_bytes());
    hasher.update(identity.inode.to_be_bytes());
    hasher.update(identity.mode.to_be_bytes());
    hasher.update(identity.links.to_be_bytes());
    hasher.update(identity.modified_seconds.to_be_bytes());
    hasher.update(identity.modified_nanos.to_be_bytes());
}

fn capture(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::CaptureFailed, cause, None)
}

fn unsupported(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::UnsupportedEntry, cause, None)
}

fn limit(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::CaptureLimit, cause, None)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn directory_swap_to_symlink_is_rejected_before_traversal() {
        use std::os::unix::fs::symlink;
        use std::time::{SystemTime, UNIX_EPOCH};

        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("visible-tree-race-{suffix}"));
        let source = root.join("src");
        let held = root.join("src-held");
        let outside = root.join("outside");
        fs::create_dir_all(&source).expect("source");
        fs::create_dir_all(&outside).expect("outside");
        fs::write(outside.join("private"), b"must not traverse").expect("outside file");
        descriptor::set_test_before_directory_open({
            let source = source.clone();
            let held = held.clone();
            let outside = outside.clone();
            move || {
                fs::rename(&source, &held).expect("hold checked directory");
                symlink(&outside, &source).expect("substitute symlink");
            }
        });
        let error = GitVisibleWorktreeShape::capture(&root, &BTreeSet::new())
            .expect_err("raced directory must refuse");
        assert_eq!(error.id(), RoutineErrorId::CaptureFailed);
        fs::remove_file(&source).expect("remove symlink");
        fs::rename(&held, &source).expect("restore directory");
        fs::remove_dir_all(root).expect("cleanup");
    }
}
