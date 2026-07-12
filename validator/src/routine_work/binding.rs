use crate::context::{LiveContext, ToolCapability};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

use super::digest::digest_of;
use super::{RoutineError, RoutineErrorId};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BoundTool {
    available: bool,
    executable: Option<String>,
    identity_sha256: String,
}

impl BoundTool {
    pub fn available(&self) -> bool {
        self.available
    }

    pub fn identity_sha256(&self) -> &str {
        &self.identity_sha256
    }

    pub(crate) fn executable(&self) -> Option<&Path> {
        self.executable.as_deref().map(Path::new)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RoutineBinding {
    binding_id: String,
    context_id: String,
    candidate_id: String,
    root_id: String,
    worktree_root: String,
    configuration_id: String,
    selected_inputs_id: String,
    tool_set_id: String,
    dirty: bool,
    tools: BTreeMap<String, BoundTool>,
}

#[derive(Serialize)]
struct BindingPayload<'a> {
    context_id: &'a str,
    candidate_id: &'a str,
    root_id: &'a str,
    worktree_root: &'a str,
    configuration_id: &'a str,
    selected_inputs_id: &'a str,
    tool_set_id: &'a str,
    dirty: bool,
}

impl RoutineBinding {
    pub fn binding_id(&self) -> &str {
        &self.binding_id
    }
    pub fn context_id(&self) -> &str {
        &self.context_id
    }
    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }
    pub fn root_id(&self) -> &str {
        &self.root_id
    }
    pub fn configuration_id(&self) -> &str {
        &self.configuration_id
    }
    pub fn selected_inputs_id(&self) -> &str {
        &self.selected_inputs_id
    }
    pub fn tool_set_id(&self) -> &str {
        &self.tool_set_id
    }
    pub fn dirty(&self) -> bool {
        self.dirty
    }
    pub fn worktree_root(&self) -> &Path {
        Path::new(&self.worktree_root)
    }
    pub fn tool(&self, name: &str) -> Option<&BoundTool> {
        self.tools.get(name)
    }

    pub(crate) fn from_live(context: &LiveContext) -> Result<Self, RoutineError> {
        let candidate_id = digest_of(context.candidate())?;
        let root_id = digest_of(context.roots())?;
        let configuration_id = digest_of(context.configuration())?;
        let selected_inputs_id = digest_of(context.selected_inputs())?;
        let tool_set_id = digest_of(context.capabilities())?;
        let tools = context
            .capabilities()
            .tools
            .iter()
            .map(tool_binding)
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let worktree_root = context
            .worktree_root()
            .to_str()
            .ok_or_else(|| {
                RoutineError::new(
                    RoutineErrorId::ContextMismatch,
                    "worktree-root-not-utf8",
                    None,
                )
            })?
            .to_owned();
        let payload = BindingPayload {
            context_id: context.context_id(),
            candidate_id: &candidate_id,
            root_id: &root_id,
            worktree_root: &worktree_root,
            configuration_id: &configuration_id,
            selected_inputs_id: &selected_inputs_id,
            tool_set_id: &tool_set_id,
            dirty: context.candidate().dirty,
        };
        Ok(Self {
            binding_id: digest_of(&payload)?,
            context_id: context.context_id().to_owned(),
            candidate_id,
            root_id,
            worktree_root,
            configuration_id,
            selected_inputs_id,
            tool_set_id,
            dirty: context.candidate().dirty,
            tools,
        })
    }
}

fn tool_binding(tool: &ToolCapability) -> Result<(String, BoundTool), RoutineError> {
    Ok((
        tool.name.clone(),
        BoundTool {
            available: tool.available,
            executable: tool.executable.clone(),
            identity_sha256: digest_of(tool)?,
        },
    ))
}
