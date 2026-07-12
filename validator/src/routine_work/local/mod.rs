mod file;
mod git;
mod tree;

#[cfg(test)]
use super::authority::test_authority_checkpoint;
use super::authority::{current_binding, ensure_unchanged};
use super::digest::sha256;
use super::{DirtyChange, DirtySnapshot, RoutineBinding, RoutineError, RoutineErrorId};
use crate::context::LiveContext;
use file::{RootAnchor, content_identity};
use git::{parse_status, status_bytes};
use tree::WorktreeShape;

pub struct LocalDirtyTree;

pub(super) struct CompleteCapture {
    binding: RoutineBinding,
    status_sha256: String,
    changes: Vec<DirtyChange>,
}

impl CompleteCapture {
    fn new(binding: RoutineBinding, status_sha256: String, changes: Vec<DirtyChange>) -> Self {
        Self {
            binding,
            status_sha256,
            changes,
        }
    }

    pub(super) fn into_parts(self) -> (RoutineBinding, String, Vec<DirtyChange>) {
        (self.binding, self.status_sha256, self.changes)
    }
}

impl LocalDirtyTree {
    pub fn capture(context: &LiveContext) -> Result<DirtySnapshot, RoutineError> {
        let binding = current_binding(context)?;
        #[cfg(test)]
        test_authority_checkpoint();
        let root = binding.worktree_root();
        let anchor = RootAnchor::capture(root)?;
        let first_shape = WorktreeShape::capture(root)?;
        let first = status_bytes(&binding)?;
        let rows = parse_status(&first)?;
        let mut changes = Vec::with_capacity(rows.len());
        for row in rows {
            let content_sha256 = if row.needs_content() {
                Some(content_identity(root, row.path())?)
            } else {
                None
            };
            changes.push(DirtyChange::new(
                row.path().clone(),
                row.previous_path().cloned(),
                row.kind(),
                content_sha256,
            )?);
        }
        anchor.revalidate(root)?;
        let second = status_bytes(&binding)?;
        let second_shape = WorktreeShape::capture(root)?;
        if first != second || first_shape != second_shape {
            return Err(RoutineError::new(
                RoutineErrorId::ConcurrentMutation,
                "worktree-changed-during-capture",
                None,
            ));
        }
        anchor.revalidate(root)?;
        ensure_unchanged(context, &binding, "routine-binding-changed-during-capture")?;
        DirtySnapshot::from_complete_capture(CompleteCapture::new(binding, sha256(&first), changes))
    }
}
