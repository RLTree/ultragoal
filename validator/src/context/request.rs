use super::bound_context::EffectClass;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// The source of one capability observation. Exact-self probes are reserved for
/// the currently executing public program; they never accept a caller path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum ToolProbe {
    Path(String),
    CurrentExecutable(String),
}

impl ToolProbe {
    pub(super) fn name(&self) -> &str {
        match self {
            Self::Path(name) | Self::CurrentExecutable(name) => name,
        }
    }
}

#[derive(Clone, Debug)]
pub struct BuildRequest {
    pub(super) start: PathBuf,
    pub(super) expected_repository_root: Option<PathBuf>,
    pub(super) expected_worktree_root: Option<PathBuf>,
    pub(super) effect: EffectClass,
    pub(super) root_grant: Option<RootEffectGrant>,
    pub(super) configuration: BTreeMap<String, String>,
    pub(super) secret_sources: BTreeMap<String, String>,
    pub(super) selected_inputs: Vec<PathBuf>,
    pub(super) tool_probes: Vec<ToolProbe>,
}

impl BuildRequest {
    pub fn new(start: impl Into<PathBuf>) -> Self {
        Self {
            start: start.into(),
            expected_repository_root: None,
            expected_worktree_root: None,
            effect: EffectClass::Read,
            root_grant: None,
            configuration: BTreeMap::new(),
            secret_sources: BTreeMap::new(),
            selected_inputs: Vec::new(),
            tool_probes: vec![ToolProbe::Path("git".to_owned())],
        }
    }

    pub fn expect_repository_root(mut self, path: impl Into<PathBuf>) -> Self {
        self.expected_repository_root = Some(path.into());
        self
    }

    pub fn expect_worktree_root(mut self, path: impl Into<PathBuf>) -> Self {
        self.expected_worktree_root = Some(path.into());
        self
    }

    pub fn with_effect(mut self, effect: EffectClass) -> Self {
        self.effect = effect;
        self
    }

    /// Root-owned construction seam for the sole public workspace writer.
    /// Issuing this structural grant performs no effect; the selected scope is
    /// still captured and revalidated by `LiveContext::build`.
    pub(crate) fn with_root_workspace_grant(mut self, write_scope: impl Into<PathBuf>) -> Self {
        self.effect = EffectClass::WorkspaceWrite;
        self.root_grant = Some(RootEffectGrant::issue(
            EffectClass::WorkspaceWrite,
            vec![write_scope.into()],
        ));
        self
    }

    pub fn bind_non_secret_configuration(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.configuration.insert(key.into(), value.into());
        self
    }

    pub fn bind_secret_source(
        mut self,
        name: impl Into<String>,
        public_version: impl Into<String>,
    ) -> Self {
        self.secret_sources
            .insert(name.into(), public_version.into());
        self
    }

    pub fn select_input(mut self, path: impl Into<PathBuf>) -> Self {
        self.selected_inputs.push(path.into());
        self
    }

    pub fn probe_tool(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        if !self.tool_probes.iter().any(|probe| probe.name() == name) {
            self.tool_probes.push(ToolProbe::Path(name));
        }
        self
    }

    /// Pins an allowlisted internal capability to this process's executable.
    /// This is intentionally not a path-taking API: repository or caller input
    /// cannot substitute an executable into a candidate-bound `LiveContext`.
    pub(crate) fn probe_current_executable(mut self, name: &'static str) -> Self {
        self.tool_probes.retain(|probe| probe.name() != name);
        self.tool_probes
            .push(ToolProbe::CurrentExecutable(name.to_owned()));
        self
    }

    #[cfg(test)]
    pub(in crate::context) fn with_root_grant(mut self, grant: RootEffectGrant) -> Self {
        self.root_grant = Some(grant);
        self
    }
}

#[derive(Clone, Debug)]
pub(in crate::context) struct RootEffectGrant {
    effect: EffectClass,
    write_scopes: Vec<PathBuf>,
}

impl RootEffectGrant {
    pub(in crate::context) fn issue(effect: EffectClass, write_scopes: Vec<PathBuf>) -> Self {
        Self {
            effect,
            write_scopes,
        }
    }

    pub(super) fn effect(&self) -> EffectClass {
        self.effect
    }
    pub(super) fn write_scopes(&self) -> &[PathBuf] {
        &self.write_scopes
    }
}
