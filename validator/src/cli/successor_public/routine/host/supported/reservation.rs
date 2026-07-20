use super::super::super::{HostFailure, ReservedCheckpoint, TerminalCheckpoint};
use super::super::HostState;
use super::checkpoint_storage::write_checkpoint;
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
        let previous = self.read_optional_checkpoint()?;
        let (generation, expected_existing) = match previous {
            None => (1, false),
            Some(previous) => {
                validate_checkpoint(&previous, request.binding, None)?;
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
        write_checkpoint(&self.adapter, &checkpoint, expected_existing)?;
        self.verify()
    }

    pub(crate) fn record_terminal_checkpoint(
        &self,
        request: TerminalCheckpoint<'_>,
    ) -> Result<(), HostFailure> {
        let previous = self.read_optional_checkpoint()?.ok_or(HostFailure::Busy)?;
        validate_checkpoint(&previous, request.binding, None)?;
        if (!previous.is_reserved() && previous.state != "reconciled")
            || previous.finding_binding.as_ref() != request.finding_binding
            || previous.continuation != request.continuation
            || previous.attempt_grant != request.attempt_grant
        {
            return Err(HostFailure::Busy);
        }
        let generation = previous
            .generation
            .checked_add(1)
            .ok_or(HostFailure::Invalid)?;
        let next = checkpoint(CheckpointDraft {
            binding: request.binding,
            continuation: request.continuation,
            recovery_marker: &previous.recovery_marker,
            attempt_grant: request.attempt_grant,
            authenticated_ledger_head: request.authenticated_ledger_head,
            finding_binding: request.finding_binding,
            terminal_outcome: Some(request.terminal_outcome),
            state: "terminal-event-pending",
            generation,
        })?;
        write_checkpoint(&self.adapter, &next, true)?;
        self.verify()
    }
}
