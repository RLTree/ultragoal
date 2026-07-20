use super::super::{HostFailure, HostState};
use super::checkpoint::ContinuationCheckpoint;
use super::checkpoint_storage::write_checkpoint;
use super::checkpoint_validation::{event_projection, terminal_event_id};
use std::path::Path;

impl HostState {
    pub(crate) fn mark_event_joined(
        &self,
        checkpoint: &ContinuationCheckpoint,
    ) -> Result<(), HostFailure> {
        let current = self
            .exact_checkpoint(
                Path::new(&checkpoint.target),
                &checkpoint.context_id,
                &checkpoint.candidate_id,
                &checkpoint.plan_id,
                &checkpoint.snapshot_id,
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
        write_checkpoint(&self.adapter, &next, true)?;
        self.verify()
    }

    pub(crate) fn mark_checkpoint_reconciled(
        &self,
        checkpoint: &ContinuationCheckpoint,
        authenticated_ledger_head: String,
    ) -> Result<(), HostFailure> {
        let current = self
            .exact_checkpoint(
                Path::new(&checkpoint.target),
                &checkpoint.context_id,
                &checkpoint.candidate_id,
                &checkpoint.plan_id,
                &checkpoint.snapshot_id,
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
        write_checkpoint(&self.adapter, &next, true)?;
        self.verify()
    }

    pub(crate) fn mark_checkpoint_ambiguous(
        &self,
        checkpoint: &ContinuationCheckpoint,
    ) -> Result<(), HostFailure> {
        let current = self
            .exact_checkpoint(
                Path::new(&checkpoint.target),
                &checkpoint.context_id,
                &checkpoint.candidate_id,
                &checkpoint.plan_id,
                &checkpoint.snapshot_id,
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
        write_checkpoint(&self.adapter, &next, true)?;
        self.verify()
    }
}
