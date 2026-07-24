use super::super::super::{CheckpointBinding, HostFailure};
use super::super::HostState;
use super::checkpoint::ContinuationCheckpoint;
use super::checkpoint_storage::{remove_reconciled_alias, stored_checkpoint, write_checkpoint};
use super::continuity_validation::{event_projection, terminal_event_id};

impl HostState {
    pub(crate) fn mark_event_joined(
        &self,
        checkpoint: &ContinuationCheckpoint,
    ) -> Result<(), HostFailure> {
        let current = self
            .exact_checkpoint(
                CheckpointBinding::new(
                    std::path::Path::new(&checkpoint.target),
                    &checkpoint.context_id,
                    &checkpoint.candidate_id,
                    &checkpoint.plan_id,
                    &checkpoint.snapshot_id,
                    checkpoint.execution_id(),
                ),
                Some(&checkpoint.continuation),
            )?
            .ok_or(HostFailure::Invalid)?;
        if current.state != "terminal-event-pending"
            || current.generation != checkpoint.generation
            || current.event_id != checkpoint.event_id
        {
            return Err(HostFailure::Busy);
        }
        let mut next = current;
        next.state = "terminal-event-joined".to_owned();
        next.generation = next.generation.checked_add(1).ok_or(HostFailure::Invalid)?;
        let location =
            stored_checkpoint(&self.adapter, binding_from(&next), Some(&next.continuation))?
                .ok_or(HostFailure::Invalid)?
                .location;
        write_checkpoint(&self.adapter, binding_from(&next), location, &next, true)?;
        remove_reconciled_alias(&self.adapter, binding_from(&next), &next)?;
        self.verify()
    }

    pub(crate) fn mark_checkpoint_reconciled(
        &self,
        checkpoint: &ContinuationCheckpoint,
        authenticated_ledger_head: String,
    ) -> Result<(), HostFailure> {
        let current = self
            .exact_checkpoint(
                CheckpointBinding::new(
                    std::path::Path::new(&checkpoint.target),
                    &checkpoint.context_id,
                    &checkpoint.candidate_id,
                    &checkpoint.plan_id,
                    &checkpoint.snapshot_id,
                    checkpoint.execution_id(),
                ),
                Some(&checkpoint.continuation),
            )?
            .ok_or(HostFailure::Invalid)?;
        if !current.is_reserved()
            || current.generation != checkpoint.generation
            || current.authenticated_ledger_head != checkpoint.authenticated_ledger_head
        {
            return Err(HostFailure::Busy);
        }
        let mut next = current;
        next.state = "reconciled".to_owned();
        next.authenticated_ledger_head = authenticated_ledger_head;
        next.generation = next.generation.checked_add(1).ok_or(HostFailure::Invalid)?;
        next.event_id = terminal_event_id(&next.continuation, &next.authenticated_ledger_head);
        next.event_sequence = next.generation;
        let location =
            stored_checkpoint(&self.adapter, binding_from(&next), Some(&next.continuation))?
                .ok_or(HostFailure::Invalid)?
                .location;
        write_checkpoint(&self.adapter, binding_from(&next), location, &next, true)?;
        self.verify()
    }

    pub(crate) fn mark_checkpoint_ambiguous(
        &self,
        checkpoint: &ContinuationCheckpoint,
    ) -> Result<(), HostFailure> {
        let current = self
            .exact_checkpoint(
                CheckpointBinding::new(
                    std::path::Path::new(&checkpoint.target),
                    &checkpoint.context_id,
                    &checkpoint.candidate_id,
                    &checkpoint.plan_id,
                    &checkpoint.snapshot_id,
                    checkpoint.execution_id(),
                ),
                Some(&checkpoint.continuation),
            )?
            .ok_or(HostFailure::Invalid)?;
        if (!current.is_reserved() && current.state != "reconciled")
            || current.generation != checkpoint.generation
        {
            return Err(HostFailure::Busy);
        }
        let mut next = current;
        next.state = "ambiguous".to_owned();
        next.terminal_outcome = Some(crate::routine_work::RoutineTerminalOutcome::Ambiguous);
        next.generation = next.generation.checked_add(1).ok_or(HostFailure::Invalid)?;
        let (status, transition) = event_projection(next.terminal_outcome);
        next.event_status = status.to_owned();
        next.event_transition = transition.to_owned();
        let location =
            stored_checkpoint(&self.adapter, binding_from(&next), Some(&next.continuation))?
                .ok_or(HostFailure::Invalid)?
                .location;
        write_checkpoint(&self.adapter, binding_from(&next), location, &next, true)?;
        self.verify()
    }
}

fn binding_from(checkpoint: &ContinuationCheckpoint) -> CheckpointBinding<'_> {
    CheckpointBinding::new(
        std::path::Path::new(&checkpoint.target),
        &checkpoint.context_id,
        &checkpoint.candidate_id,
        &checkpoint.plan_id,
        &checkpoint.snapshot_id,
        checkpoint.execution_id(),
    )
}
