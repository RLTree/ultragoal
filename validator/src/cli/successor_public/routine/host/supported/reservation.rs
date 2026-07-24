use super::super::super::{HostFailure, ReservedCheckpoint, TerminalCheckpoint};
use super::super::HostState;
use super::checkpoint_storage::{
    CheckpointLocation, preserve_reconciled_handoff, stored_checkpoint, write_checkpoint,
};
use super::continuity_validation::{CheckpointDraft, checkpoint, validate_checkpoint};

impl HostState {
    pub(crate) fn record_reserved_checkpoint(
        &self,
        request: ReservedCheckpoint<'_>,
    ) -> Result<(), HostFailure> {
        if !request.binding.target().is_absolute()
            || request.binding.target().to_str().is_none()
            || !request.continuation.starts_with("routine-cont-")
            || request.recovery_marker.is_empty()
            || request.attempt_grant.is_empty()
            || request.authenticated_ledger_head.is_empty()
        {
            return Err(HostFailure::Invalid);
        }
        let previous = stored_checkpoint(&self.adapter, request.binding, None)?;
        let (generation, location, expected_existing) = match previous {
            None => (1, CheckpointLocation::Canonical, false),
            Some(previous) => {
                validate_checkpoint(&previous.checkpoint, request.binding, None)?;
                if previous.checkpoint.state != "reconciled" {
                    return Err(HostFailure::Busy);
                }
                if previous.location == CheckpointLocation::Canonical {
                    preserve_reconciled_handoff(
                        &self.adapter,
                        request.binding,
                        &previous.checkpoint,
                    )?;
                }
                // Reconciliation is terminal for the prior reservation: the private
                // custody ledger retains its no-effect outcome. Preserve the caller's
                // binding-specific continuation in the canonical directory before
                // replacing the primary record; the singleton legacy filename can
                // legitimately hold another binding's history.
                (
                    previous
                        .checkpoint
                        .generation
                        .checked_add(1)
                        .ok_or(HostFailure::Invalid)?,
                    previous.location,
                    true,
                )
            }
        };
        let checkpoint = checkpoint(CheckpointDraft {
            binding: request.binding,
            continuation: request.continuation,
            recovery_marker: request.recovery_marker,
            attempt_grant: request.attempt_grant,
            authenticated_ledger_head: request.authenticated_ledger_head,
            finding_binding: request.finding_binding,
            terminal_outcome: None,
            state: "reserved",
            generation,
        })?;
        write_checkpoint(
            &self.adapter,
            request.binding,
            location,
            &checkpoint,
            expected_existing,
        )?;
        self.verify()
    }

    pub(crate) fn record_terminal_checkpoint(
        &self,
        request: TerminalCheckpoint<'_>,
    ) -> Result<(), HostFailure> {
        let previous =
            stored_checkpoint(&self.adapter, request.binding, None)?.ok_or(HostFailure::Busy)?;
        validate_checkpoint(&previous.checkpoint, request.binding, None)?;
        if (!previous.checkpoint.is_reserved() && previous.checkpoint.state != "reconciled")
            || previous.checkpoint.finding_binding.as_ref() != request.finding_binding
            || previous.checkpoint.continuation != request.continuation
            || previous.checkpoint.attempt_grant != request.attempt_grant
        {
            return Err(HostFailure::Busy);
        }
        let generation = previous
            .checkpoint
            .generation
            .checked_add(1)
            .ok_or(HostFailure::Invalid)?;
        let next = checkpoint(CheckpointDraft {
            binding: request.binding,
            continuation: request.continuation,
            recovery_marker: &previous.checkpoint.recovery_marker,
            attempt_grant: request.attempt_grant,
            authenticated_ledger_head: request.authenticated_ledger_head,
            finding_binding: request.finding_binding,
            terminal_outcome: Some(request.terminal_outcome),
            state: "terminal-event-pending",
            generation,
        })?;
        write_checkpoint(
            &self.adapter,
            request.binding,
            previous.location,
            &next,
            true,
        )?;
        self.verify()
    }
}
