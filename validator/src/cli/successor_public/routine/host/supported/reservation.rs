use super::super::{HostFailure, HostState};
use super::checkpoint_storage::write_checkpoint;
use super::continuity_validation::{checkpoint, validate_checkpoint};
use std::path::Path;

impl HostState {
    pub(crate) fn record_reserved_checkpoint(
        &self,
        target: &Path,
        context_id: &str,
        candidate_id: &str,
        plan_id: &str,
        snapshot_id: &str,
        continuation: &str,
        recovery_marker: &str,
        attempt_grant: &str,
        authenticated_ledger_head: &str,
        finding_binding: Option<&crate::state::RoutineFindingBinding>,
    ) -> Result<(), HostFailure> {
        if !target.is_absolute()
            || target.to_str().is_none()
            || !continuation.starts_with("routine-cont-")
            || recovery_marker.is_empty()
            || attempt_grant.is_empty()
            || authenticated_ledger_head.is_empty()
        {
            return Err(HostFailure::Invalid);
        }
        let previous = self.read_optional_checkpoint()?;
        let (generation, expected_existing) = match previous {
            None => (1, false),
            Some(previous) => {
                validate_checkpoint(
                    &previous,
                    target,
                    context_id,
                    candidate_id,
                    plan_id,
                    snapshot_id,
                    None,
                )?;
                if previous.state != "reconciled" {
                    return Err(HostFailure::Busy);
                }
                (
                    previous
                        .generation
                        .checked_add(1)
                        .ok_or(HostFailure::Invalid)?,
                    true,
                )
            }
        };
        let checkpoint = checkpoint(
            target,
            context_id,
            candidate_id,
            plan_id,
            snapshot_id,
            continuation,
            recovery_marker,
            attempt_grant,
            authenticated_ledger_head,
            finding_binding,
            None,
            "reserved",
            generation,
        )?;
        write_checkpoint(&self.adapter, &checkpoint, expected_existing)?;
        self.verify()
    }

    pub(crate) fn record_terminal_checkpoint(
        &self,
        target: &Path,
        context_id: &str,
        candidate_id: &str,
        plan_id: &str,
        snapshot_id: &str,
        continuation: &str,
        attempt_grant: &str,
        authenticated_ledger_head: &str,
        finding_binding: Option<&crate::state::RoutineFindingBinding>,
        terminal_outcome: crate::routine_work::RoutineTerminalOutcome,
    ) -> Result<(), HostFailure> {
        let previous = self.read_optional_checkpoint()?.ok_or(HostFailure::Busy)?;
        validate_checkpoint(
            &previous,
            target,
            context_id,
            candidate_id,
            plan_id,
            snapshot_id,
            None,
        )?;
        if (!previous.is_reserved() && previous.state != "reconciled")
            || previous.finding_binding.as_ref() != finding_binding
            || previous.continuation != continuation
            || previous.attempt_grant != attempt_grant
        {
            return Err(HostFailure::Busy);
        }
        let generation = previous
            .generation
            .checked_add(1)
            .ok_or(HostFailure::Invalid)?;
        let next = checkpoint(
            target,
            context_id,
            candidate_id,
            plan_id,
            snapshot_id,
            continuation,
            &previous.recovery_marker,
            attempt_grant,
            authenticated_ledger_head,
            finding_binding,
            Some(terminal_outcome),
            "terminal-event-pending",
            generation,
        )?;
        write_checkpoint(&self.adapter, &next, true)?;
        self.verify()
    }
}
