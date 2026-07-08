use super::super::command_failure::CommandFailureSummary;

#[derive(Clone, Debug)]
pub(super) struct FullCommandRun {
    pub(super) exit_code: i32,
    pub(super) status_success: bool,
    pub(super) launch_error: bool,
    pub(super) duration_ms: u64,
    pub(super) stdout_digest: String,
    pub(super) stderr_digest: String,
    pub(super) failure: CommandFailureSummary,
}
