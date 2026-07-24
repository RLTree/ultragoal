use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum RoutineCheckpointOperation {
    Terminal,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ContinuationCheckpoint {
    pub(super) schema_version: String,
    pub(super) generation: u64,
    pub(super) target: String,
    pub(super) context_id: String,
    pub(super) candidate_id: String,
    pub(super) plan_id: String,
    pub(super) snapshot_id: String,
    pub(super) continuation: String,
    pub(super) recovery_marker: String,
    pub(super) attempt_grant: String,
    pub(super) authenticated_ledger_head: String,
    pub(super) finding_binding: Option<crate::state::RoutineFindingBinding>,
    pub(super) operation: RoutineCheckpointOperation,
    pub(super) terminal_outcome: Option<crate::routine_work::RoutineTerminalOutcome>,
    pub(super) state: String,
    pub(super) event_id: String,
    pub(super) event_observed_at_unix_ms: u64,
    pub(super) event_sequence: u64,
    pub(super) event_parent_id: Option<String>,
    pub(super) event_status: String,
    pub(super) event_transition: String,
}

impl ContinuationCheckpoint {
    pub(crate) fn target(&self) -> &str {
        &self.target
    }

    pub(crate) fn context_id(&self) -> &str {
        &self.context_id
    }

    pub(crate) fn candidate_id(&self) -> &str {
        &self.candidate_id
    }

    pub(crate) fn plan_id(&self) -> &str {
        &self.plan_id
    }

    pub(crate) fn snapshot_id(&self) -> &str {
        &self.snapshot_id
    }

    pub(crate) fn ledger_head(&self) -> &str {
        &self.authenticated_ledger_head
    }

    pub(crate) fn attempt_grant(&self) -> &str {
        &self.attempt_grant
    }

    pub(crate) fn recovery_marker(&self) -> &str {
        &self.recovery_marker
    }

    pub(crate) fn finding_binding(&self) -> Option<&crate::state::RoutineFindingBinding> {
        self.finding_binding.as_ref()
    }

    pub(crate) fn is_reserved(&self) -> bool {
        self.state == "reserved"
    }

    pub(crate) fn is_complete(&self) -> bool {
        self.is_terminal()
            && self.terminal_outcome == Some(crate::routine_work::RoutineTerminalOutcome::Complete)
    }

    pub(crate) fn is_terminal(&self) -> bool {
        matches!(
            self.state.as_str(),
            "terminal-event-pending" | "terminal-event-joined"
        )
    }

    pub(crate) fn state(&self) -> &str {
        &self.state
    }

    pub(crate) fn continuation(&self) -> &str {
        &self.continuation
    }

    pub(crate) fn event_id(&self) -> &str {
        &self.event_id
    }

    pub(crate) fn event_observed_at_unix_ms(&self) -> u64 {
        self.event_observed_at_unix_ms
    }

    pub(crate) fn event_sequence(&self) -> u64 {
        self.event_sequence
    }

    pub(crate) fn event_parent_id(&self) -> Option<&str> {
        self.event_parent_id.as_deref()
    }

    pub(crate) fn event_status(&self) -> &str {
        &self.event_status
    }

    pub(crate) fn event_transition(&self) -> &str {
        &self.event_transition
    }

    pub(crate) fn terminal_outcome(&self) -> Option<crate::routine_work::RoutineTerminalOutcome> {
        self.terminal_outcome
    }
}
