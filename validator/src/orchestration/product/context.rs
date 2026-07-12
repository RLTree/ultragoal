use super::ProductError;
use crate::orchestration::{
    Actor, Binding, EffectReceipt, EffectRequest, EffectSink, JournalHead, OrchestrationError,
    Orchestrator, ScopePolicy, WorkGraph,
};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Component, Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

#[derive(Clone, Debug)]
pub struct ProductContext {
    pub(crate) graph: WorkGraph,
    pub(crate) policy: ScopePolicy,
    pub(crate) binding: Binding,
    pub(crate) root: Actor,
}

impl ProductContext {
    pub fn new(graph: WorkGraph, policy: ScopePolicy, binding: Binding, root: Actor) -> Self {
        Self {
            graph,
            policy,
            binding,
            root,
        }
    }

    pub fn binding(&self) -> &Binding {
        &self.binding
    }

    pub fn root(&self) -> &Actor {
        &self.root
    }
}

#[derive(Clone, Debug)]
pub struct ProductWorkspace {
    supplied_root: PathBuf,
    canonical_root: PathBuf,
    identity: String,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
}

impl ProductWorkspace {
    /// Anchors one exact journal directory without creating or changing it.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, ProductError> {
        let supplied = root.as_ref();
        validate_absolute_path(supplied)?;
        let metadata =
            fs::symlink_metadata(supplied).map_err(|_| ProductError::InvalidWorkspacePath)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(ProductError::InvalidWorkspacePath);
        }
        let canonical_root =
            fs::canonicalize(supplied).map_err(|_| ProductError::InvalidWorkspacePath)?;
        let identity = workspace_identity(&canonical_root, &metadata)?;
        let workspace = Self {
            supplied_root: supplied.to_path_buf(),
            canonical_root,
            identity,
            #[cfg(unix)]
            device: metadata.dev(),
            #[cfg(unix)]
            inode: metadata.ino(),
        };
        workspace.verify()?;
        Ok(workspace)
    }

    pub fn identity(&self) -> &str {
        &self.identity
    }

    pub(crate) fn root(&self) -> &Path {
        &self.supplied_root
    }

    pub(crate) fn verify(&self) -> Result<(), ProductError> {
        let metadata = fs::symlink_metadata(&self.supplied_root)
            .map_err(|_| ProductError::WorkspaceChanged)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(ProductError::WorkspaceChanged);
        }
        let canonical =
            fs::canonicalize(&self.supplied_root).map_err(|_| ProductError::WorkspaceChanged)?;
        if canonical != self.canonical_root
            || workspace_identity(&canonical, &metadata)? != self.identity
        {
            return Err(ProductError::WorkspaceChanged);
        }
        #[cfg(unix)]
        if metadata.dev() != self.device || metadata.ino() != self.inode {
            return Err(ProductError::WorkspaceChanged);
        }
        Ok(())
    }
}

pub(crate) fn open_engine<S: EffectSink>(
    context: &ProductContext,
    workspace: &ProductWorkspace,
    expected_head: &JournalHead,
    sink: S,
) -> Result<Orchestrator<S>, ProductError> {
    workspace.verify()?;
    if expected_head.binding != context.binding {
        return Err(ProductError::StaleCandidate);
    }
    let engine = Orchestrator::restart_durable(
        context.graph.clone(),
        context.policy.clone(),
        expected_head.clone(),
        context.root.clone(),
        workspace.root(),
        sink,
    )
    .map_err(ProductError::from)?;
    workspace.verify()?;
    Ok(engine)
}

pub fn journal_head_identity(head: &JournalHead) -> Result<String, ProductError> {
    let bytes = serde_json::to_vec(head).map_err(|_| ProductError::StaleCandidate)?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ReadOnlySink;

impl EffectSink for ReadOnlySink {
    fn apply(&mut self, _: &EffectRequest) -> Result<EffectReceipt, OrchestrationError> {
        Err(OrchestrationError::EffectDenied)
    }
}

fn validate_absolute_path(path: &Path) -> Result<(), ProductError> {
    if !path.is_absolute()
        || path.as_os_str().is_empty()
        || path.to_str().is_none()
        || path.components().any(|component| {
            matches!(
                component,
                Component::CurDir | Component::ParentDir | Component::Prefix(_)
            )
        })
    {
        return Err(ProductError::InvalidWorkspacePath);
    }
    Ok(())
}

fn workspace_identity(path: &Path, metadata: &fs::Metadata) -> Result<String, ProductError> {
    let path = path
        .to_str()
        .ok_or(ProductError::InvalidWorkspacePath)?
        .as_bytes();
    let mut hasher = Sha256::new();
    hasher.update(b"harness-ultragoal/orchestration-workspace/v1\0");
    hasher.update((path.len() as u64).to_be_bytes());
    hasher.update(path);
    #[cfg(unix)]
    {
        hasher.update(metadata.dev().to_be_bytes());
        hasher.update(metadata.ino().to_be_bytes());
    }
    #[cfg(not(unix))]
    {
        hasher.update(metadata.len().to_be_bytes());
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}
