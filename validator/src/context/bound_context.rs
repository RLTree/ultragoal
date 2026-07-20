use super::error::ContextError;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[cfg(unix)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct WorktreeDirectoryIdentity {
    device: u64,
    inode: u64,
}

#[cfg(unix)]
impl WorktreeDirectoryIdentity {
    pub(super) fn new(device: u64, inode: u64) -> Self {
        Self { device, inode }
    }

    pub(crate) fn matches(self, device: u64, inode: u64) -> bool {
        self.device == device && self.inode == inode
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EffectClass {
    Read,
    PlannedWrite,
    WorkspaceWrite,
    ExternalWrite,
    Destructive,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RootIdentity {
    pub repository_root: String,
    pub worktree_root: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CandidateIdentity {
    pub head_commit: Option<String>,
    pub head_tree: Option<String>,
    pub branch: Option<String>,
    pub status_sha256: String,
    pub worktree_diff_sha256: String,
    pub staged_diff_sha256: String,
    pub untracked_content_sha256: String,
    pub dirty: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ToolCapability {
    pub name: String,
    pub available: bool,
    pub executable: Option<String>,
    pub executable_sha256: Option<String>,
    pub byte_length: Option<u64>,
    pub unix_mode: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CapabilitySet {
    pub path_search_sha256: String,
    pub tools: Vec<ToolCapability>,
}

impl CapabilitySet {
    pub fn tool(&self, name: &str) -> Option<&ToolCapability> {
        self.tools.iter().find(|tool| tool.name == name)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ConfigurationIdentity {
    pub public_values: BTreeMap<String, String>,
    pub secret_sources: Vec<SecretSourceIdentity>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SecretSourceIdentity {
    pub name: String,
    pub public_version: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PermissionIdentity {
    pub repository_metadata_read_succeeded: bool,
    pub worktree_metadata_read_succeeded: bool,
    pub repository_directory_open_succeeded: bool,
    pub worktree_directory_open_succeeded: bool,
    pub repository_unix_mode: Option<u32>,
    pub worktree_unix_mode: Option<u32>,
    pub write_probe_performed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SelectedInputIdentity {
    pub relative_path: String,
    pub sha256: String,
    pub byte_length: u64,
    pub unix_mode: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EffectBoundary {
    pub selected: EffectClass,
    pub permitted: BTreeSet<EffectClass>,
    pub write_scopes: Vec<String>,
}

impl EffectBoundary {
    pub fn authorize(&self, requested: EffectClass) -> Result<(), ContextError> {
        let structurally_covered = matches!(
            (self.selected, requested),
            (EffectClass::Read, EffectClass::Read)
                | (
                    EffectClass::PlannedWrite,
                    EffectClass::Read | EffectClass::PlannedWrite
                )
                | (
                    EffectClass::WorkspaceWrite,
                    EffectClass::Read | EffectClass::PlannedWrite | EffectClass::WorkspaceWrite
                )
                | (
                    EffectClass::ExternalWrite,
                    EffectClass::Read | EffectClass::ExternalWrite
                )
                | (
                    EffectClass::Destructive,
                    EffectClass::Read
                        | EffectClass::PlannedWrite
                        | EffectClass::WorkspaceWrite
                        | EffectClass::Destructive
                )
        );
        if !structurally_covered {
            return Err(ContextError::EffectDenied(format!(
                "operation requested {requested:?}, context is structurally bounded at {selected:?}",
                selected = self.selected
            )));
        }
        if !self.permitted.contains(&requested) {
            return Err(ContextError::EffectDenied(format!(
                "{requested:?} was not permitted"
            )));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct ContextPayload {
    schema_version: &'static str,
    roots: RootIdentity,
    candidate: CandidateIdentity,
    configuration: ConfigurationIdentity,
    capabilities: CapabilitySet,
    permissions: PermissionIdentity,
    effect: EffectBoundary,
    selected_inputs: Vec<SelectedInputIdentity>,
}

pub(super) struct ContextPayloadInput {
    pub(super) roots: RootIdentity,
    pub(super) candidate: CandidateIdentity,
    pub(super) configuration: ConfigurationIdentity,
    pub(super) capabilities: CapabilitySet,
    pub(super) permissions: PermissionIdentity,
    pub(super) effect: EffectBoundary,
    pub(super) selected_inputs: Vec<SelectedInputIdentity>,
}

impl ContextPayload {
    pub(super) fn new(input: ContextPayloadInput) -> Self {
        Self {
            schema_version: "LiveContext-v1",
            roots: input.roots,
            candidate: input.candidate,
            configuration: input.configuration,
            capabilities: input.capabilities,
            permissions: input.permissions,
            effect: input.effect,
            selected_inputs: input.selected_inputs,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct LiveContext {
    context_id: String,
    #[serde(flatten)]
    payload: ContextPayload,
    #[cfg(unix)]
    #[serde(skip)]
    worktree_directory_identity: WorktreeDirectoryIdentity,
}

impl LiveContext {
    pub fn context_id(&self) -> &str {
        &self.context_id
    }
    pub fn roots(&self) -> &RootIdentity {
        &self.payload.roots
    }
    pub fn candidate(&self) -> &CandidateIdentity {
        &self.payload.candidate
    }
    pub fn configuration(&self) -> &ConfigurationIdentity {
        &self.payload.configuration
    }
    pub fn capabilities(&self) -> &CapabilitySet {
        &self.payload.capabilities
    }
    pub fn permissions(&self) -> &PermissionIdentity {
        &self.payload.permissions
    }
    pub fn effect(&self) -> &EffectBoundary {
        &self.payload.effect
    }
    pub fn selected_inputs(&self) -> &[SelectedInputIdentity] {
        &self.payload.selected_inputs
    }
    pub fn worktree_root(&self) -> &Path {
        Path::new(&self.payload.roots.worktree_root)
    }

    #[cfg(unix)]
    pub(crate) fn matches_worktree_directory(self: &Self, device: u64, inode: u64) -> bool {
        self.worktree_directory_identity.matches(device, inode)
    }

    pub(super) fn from_payload(
        payload: ContextPayload,
        context_id: String,
        #[cfg(unix)] worktree_directory_identity: WorktreeDirectoryIdentity,
    ) -> Self {
        Self {
            context_id,
            payload,
            #[cfg(unix)]
            worktree_directory_identity,
        }
    }
}
