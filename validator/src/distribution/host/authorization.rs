#[derive(Debug)]
pub struct HostAuthorization {
    pub(super) context_id: String,
    pub(super) candidate_id: String,
    pub(super) plan_sha256: String,
    pub(super) consumed: bool,
}

impl HostAuthorization {
    pub fn new(
        context_id: String,
        candidate_id: String,
        plan_sha256: String,
    ) -> Result<Self, DistributionError> {
        if !digest(&context_id) || !digest(&candidate_id) || !digest(&plan_sha256) {
            return Err(error(DistributionErrorId::InvalidSpec));
        }
        Ok(Self {
            context_id,
            candidate_id,
            plan_sha256,
            consumed: false,
        })
    }
}

pub struct CommandOutput {
    pub exit_code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostExecutorError {
    BackendFailure,
    TimedOut,
    Interrupted,
    InvalidPolicy,
}

pub trait HostExecutor {
    /// Execute the complete typed command policy.
    ///
    /// Implementations must enforce the command's scrubbed environment,
    /// timeout, and attempt bound. A backend that cannot enforce one of those
    /// fields returns `InvalidPolicy` instead of using an untyped fallback.
    fn execute_with_policy(
        &mut self,
        command: &HostCommand,
    ) -> Result<CommandOutput, HostExecutorError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HostCommandExit {
    NonZero,
    OutputLimit,
    BackendFailure,
    TimedOut,
    Interrupted,
    ReplayRejected,
    EffectUnavailable,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct HostCommandFailure {
    failed_index: usize,
    completed_count: usize,
    exit: HostCommandExit,
    retry_allowed: bool,
}

impl HostCommandFailure {
    pub const fn failed_index(&self) -> usize {
        self.failed_index
    }

    pub const fn completed_count(&self) -> usize {
        self.completed_count
    }

    pub const fn exit(&self) -> HostCommandExit {
        self.exit
    }

    pub const fn retry_allowed(&self) -> bool {
        self.retry_allowed
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct HostExecutionSnapshot {
    context_id: String,
    candidate_id: String,
    plan_sha256: String,
    command_count: usize,
    output_sha256: Vec<String>,
}

impl HostExecutionSnapshot {
    pub fn context_id(&self) -> &str {
        &self.context_id
    }

    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }

    pub fn plan_sha256(&self) -> &str {
        &self.plan_sha256
    }

    pub const fn command_count(&self) -> usize {
        self.command_count
    }

    pub fn output_sha256(&self) -> &[String] {
        &self.output_sha256
    }
}
