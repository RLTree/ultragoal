use super::bound_context::{LiveContext, RootIdentity};
use super::build::{path_text, permissions, selected_inputs};
use super::capability;
use super::error::ContextError;
use super::git;
use std::path::{Path, PathBuf};

#[cfg(unix)]
fn worktree_identity_matches(context: &LiveContext, worktree: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;

    std::fs::symlink_metadata(worktree).is_ok_and(|metadata| {
        metadata.is_dir()
            && !metadata.file_type().is_symlink()
            && context.matches_worktree_directory(metadata.dev(), metadata.ino())
    })
}

fn changed(dimension: &str) -> ContextError {
    ContextError::ConcurrentMutation(format!("{dimension} changed after context construction"))
}

impl LiveContext {
    /// Re-runs the complete read-only live snapshot before authority-bearing use.
    pub fn revalidate(&self) -> Result<(), ContextError> {
        let recorded_git = self
            .capabilities()
            .tool("git")
            .filter(|tool| tool.available)
            .ok_or_else(|| changed("Git substrate"))?;
        let git_path = PathBuf::from(
            recorded_git
                .executable
                .as_deref()
                .ok_or_else(|| changed("Git substrate path"))?,
        );
        if capability::executable_identity("git", &git_path)? != *recorded_git {
            return Err(changed("Git substrate identity"));
        }
        let (repository, worktree) = git::resolve_roots(self.worktree_root(), &git_path)?;
        let roots = RootIdentity {
            repository_root: path_text(&repository)?,
            worktree_root: path_text(&worktree)?,
        };
        if &roots != self.roots() {
            return Err(changed("repository or worktree root"));
        }
        #[cfg(unix)]
        if !worktree_identity_matches(self, &worktree) {
            return Err(changed("worktree directory identity"));
        }
        if &git::capture_candidate(&git_path, &worktree)? != self.candidate() {
            return Err(changed("Git candidate identity"));
        }
        let input_paths = self
            .selected_inputs()
            .iter()
            .map(|input| PathBuf::from(&input.relative_path))
            .collect::<Vec<_>>();
        if selected_inputs(&input_paths, &worktree)? != self.selected_inputs() {
            return Err(changed("selected inputs"));
        }
        if capability::capture(self.capability_probes(), &git_path)? != *self.capabilities() {
            return Err(changed("capabilities or PATH"));
        }
        if permissions(&repository, &worktree) != *self.permissions() {
            return Err(changed("observed root metadata"));
        }
        if capability::executable_identity("git", Path::new(&git_path))? != *recorded_git {
            return Err(changed("Git substrate identity"));
        }
        Ok(())
    }
}
