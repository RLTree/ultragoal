use crate::state::RoutineFindingBinding;
use std::path::Path;

#[derive(Clone, Copy)]
pub(crate) struct CheckpointBinding<'a> {
    target: &'a Path,
    context_id: &'a str,
    candidate_id: &'a str,
    plan_id: &'a str,
    snapshot_id: &'a str,
    execution_id: &'a str,
}

impl<'a> CheckpointBinding<'a> {
    pub(crate) fn new(
        target: &'a Path,
        context_id: &'a str,
        candidate_id: &'a str,
        plan_id: &'a str,
        snapshot_id: &'a str,
        execution_id: &'a str,
    ) -> Self {
        Self {
            target,
            context_id,
            candidate_id,
            plan_id,
            snapshot_id,
            execution_id,
        }
    }

    pub(crate) fn target(self) -> &'a Path {
        self.target
    }

    pub(crate) fn context_id(self) -> &'a str {
        self.context_id
    }

    pub(crate) fn candidate_id(self) -> &'a str {
        self.candidate_id
    }

    pub(crate) fn plan_id(self) -> &'a str {
        self.plan_id
    }

    pub(crate) fn snapshot_id(self) -> &'a str {
        self.snapshot_id
    }

    pub(crate) fn execution_id(self) -> &'a str {
        self.execution_id
    }

    pub(crate) fn without_execution_id(self) -> Self {
        Self::new(
            self.target,
            self.context_id,
            self.candidate_id,
            self.plan_id,
            self.snapshot_id,
            "",
        )
    }
}

pub(crate) struct ReservedCheckpoint<'a> {
    pub(crate) binding: CheckpointBinding<'a>,
    pub(crate) continuation: &'a str,
    pub(crate) recovery_marker: &'a str,
    pub(crate) attempt_grant: &'a str,
    pub(crate) authenticated_ledger_head: &'a str,
    pub(crate) finding_binding: Option<&'a RoutineFindingBinding>,
}

pub(crate) struct TerminalCheckpoint<'a> {
    pub(crate) binding: CheckpointBinding<'a>,
    pub(crate) continuation: &'a str,
    pub(crate) attempt_grant: &'a str,
    pub(crate) authenticated_ledger_head: &'a str,
    pub(crate) finding_binding: Option<&'a RoutineFindingBinding>,
    pub(crate) terminal_outcome: crate::routine_work::RoutineTerminalOutcome,
}
