use super::*;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct RoutineReadAncestor {
    pub(crate) relative_directory: String,
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) unix_mode: u32,
    pub(crate) owner_user_id: u32,
    pub(crate) owner_group_id: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct RoutineReadSource {
    pub(crate) relative_path: RepoPath,
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) unix_mode: u32,
    pub(crate) owner_user_id: u32,
    pub(crate) owner_group_id: u32,
    pub(crate) link_count: u64,
    pub(crate) byte_length: u64,
    pub(crate) modified_seconds: i64,
    pub(crate) modified_nanos: i64,
    pub(crate) changed_seconds: i64,
    pub(crate) changed_nanos: i64,
    pub(crate) sha256: String,
    pub(crate) ancestors: Vec<RoutineReadAncestor>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub(crate) struct RoutineInvocationSpec {
    pub(crate) node_id: String,
    pub(crate) behavior_id: String,
    pub(crate) tool_name: String,
    pub(crate) tool_identity_sha256: String,
    pub(crate) program_path_hex: String,
    pub(crate) program_sha256: String,
    pub(crate) program_byte_length: u64,
    pub(crate) program_unix_mode: Option<u32>,
    pub(crate) arguments: Vec<String>,
    pub(crate) environment_sha256: String,
    #[serde(skip)]
    pub(crate) environment: BTreeMap<String, String>,
    pub(crate) read_authority_sha256: String,
    #[serde(skip)]
    pub(crate) read_sources: Vec<RoutineReadSource>,
    pub(crate) timeout_ms: u64,
    pub(crate) output_budget_bytes: u64,
    pub(crate) declared_output_scopes: Vec<RepoPath>,
}

impl RoutineInvocationSpec {
    pub(crate) fn node_id(&self) -> &str {
        &self.node_id
    }

    #[cfg(test)]
    pub(crate) fn test_with_program_sha256(mut self, identity: impl Into<String>) -> Self {
        self.program_sha256 = identity.into();
        self
    }

    #[cfg(test)]
    pub(crate) fn test_with_program_path_hex(mut self, path: impl Into<String>) -> Self {
        self.program_path_hex = path.into();
        self
    }

    #[cfg(test)]
    pub(crate) fn test_with_behavior_id(mut self, behavior: impl Into<String>) -> Self {
        self.behavior_id = behavior.into();
        self
    }

    #[cfg(test)]
    pub(crate) fn test_with_arguments(mut self, arguments: Vec<String>) -> Self {
        self.arguments = arguments;
        self
    }

    #[cfg(test)]
    pub(crate) fn test_with_environment(mut self, environment: BTreeMap<String, String>) -> Self {
        self.environment = environment;
        self
    }

    #[cfg(test)]
    pub(crate) fn test_with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct RoutineAdapterSpec {
    pub(crate) result_scope: String,
    pub(crate) invocations: Vec<RoutineInvocationSpec>,
}

impl RoutineAdapterSpec {
    pub(crate) fn new(
        result_scope: impl Into<String>,
        invocations: Vec<RoutineInvocationSpec>,
    ) -> Self {
        Self {
            result_scope: result_scope.into(),
            invocations,
        }
    }
}
