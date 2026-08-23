use super::HostCustodyIssuance;
use super::*;
use crate::routine_work::RoutineCustodyCapability;
use sha2::{Digest, Sha256};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HostFailure {
    #[cfg(not(target_vendor = "apple"))]
    Unsupported,
    Unavailable,
    Busy,
    Invalid,
}

pub(crate) struct HostState {
    #[cfg(target_vendor = "apple")]
    pub(crate) inner: supported::HostState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RoutineStateMigrationAdmission {
    pub(crate) status: &'static str,
    pub(crate) format_status: &'static str,
    pub(crate) legacy_singleton_count: usize,
    pub(crate) canonical_continuation_count: usize,
    pub(crate) event_journal_count: usize,
    pub(crate) history_relation: &'static str,
    pub(crate) next_action: &'static str,
    pub(crate) reserved_recovery: Option<RoutineReservedRecoveryAdmission>,
    pub(crate) quarantine_plan: Option<RoutineStateQuarantinePlan>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RoutineReservedRecoveryAdmission {
    pub(crate) status: &'static str,
    pub(crate) owner_observation: &'static str,
    pub(crate) effect_evidence: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RoutineStateQuarantinePlan {
    pub(crate) plan_id: String,
    pub(crate) source_inventory_sha256: String,
    pub(crate) history_relation: &'static str,
    pub(crate) authoritative_history: &'static str,
    pub(crate) source_owner: &'static str,
    pub(crate) quarantine_owner: String,
    pub(crate) target_format: &'static str,
    pub(crate) strategy: &'static str,
    pub(crate) apply_capability: &'static str,
    pub(crate) operations: Vec<&'static str>,
    pub(crate) rollback: &'static str,
}

pub(crate) struct HostEventStore {
    #[cfg(target_vendor = "apple")]
    inner: supported::HostEventStore,
    expected: Vec<ExpectedTerminalEvent>,
}

struct ExpectedTerminalEvent {
    event: crate::observability::SemanticEvent,
    required: bool,
}

impl HostEventStore {
    pub(crate) fn query(
        &self,
        query: &crate::observability::EventQuery,
    ) -> Result<Vec<crate::observability::SemanticEvent>, String> {
        #[cfg(target_vendor = "apple")]
        {
            self.inner.store.query_validated(query, |events| {
                validate_expected_events(events, &self.expected)
            })
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = query;
            unreachable!("unsupported host event store cannot be constructed")
        }
    }

    pub(crate) fn explain(
        &self,
        query: &crate::observability::EventQuery,
        event_id: &str,
    ) -> Result<crate::observability::CausalExplanation, String> {
        #[cfg(target_vendor = "apple")]
        {
            self.inner
                .store
                .explain_validated(query, event_id, |events| {
                    validate_expected_events(events, &self.expected)
                })
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (query, event_id);
            unreachable!("unsupported host event store cannot be constructed")
        }
    }

    fn validate_all(&self) -> Result<(), String> {
        let binding = self
            .expected
            .first()
            .ok_or_else(|| "observe-store-corrupt:missing-terminal-authority".to_owned())?;
        let query = crate::observability::EventQuery::new(
            binding.event.context_id(),
            binding.event.candidate_id(),
            binding.event.source_id(),
        )?
        .limit(crate::observability::EventStore::supported_result_limit())?;
        self.query(&query).map(|_| ())
    }
}

impl HostState {
    pub(crate) fn assess_migration_admission(
        home: &Path,
        binding: CheckpointBinding<'_>,
    ) -> Result<Option<RoutineStateMigrationAdmission>, HostFailure> {
        #[cfg(target_vendor = "apple")]
        {
            supported::assess_migration_admission(home, binding)
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (home, binding);
            Err(HostFailure::Unsupported)
        }
    }

    pub(crate) fn open_existing_for_target(
        home: &Path,
        target: &Path,
    ) -> Result<Self, HostFailure> {
        #[cfg(target_vendor = "apple")]
        {
            Ok(Self {
                inner: supported::HostState::open_existing_for_target(home, target)?,
            })
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (home, target);
            Err(HostFailure::Unsupported)
        }
    }

    fn open_event_store(
        &self,
        target: &Path,
        context: &crate::context::LiveContext,
        source_id: &str,
        create: bool,
    ) -> Result<Option<HostEventStore>, HostFailure> {
        let binding = crate::observability::SemanticEvent::for_context(
            context,
            source_id,
            "host-event-store-binding",
            0,
            0,
            "observe.store",
            "unknown",
        )
        .map_err(|_| HostFailure::Invalid)?;
        #[cfg(target_vendor = "apple")]
        {
            self.inner
                .open_event_store(
                    target,
                    binding.context_id(),
                    binding.candidate_id(),
                    binding.source_id(),
                    create,
                )
                .map(|store| {
                    store.map(|inner| HostEventStore {
                        inner,
                        expected: Vec::new(),
                    })
                })
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (target, context, source_id, create);
            unreachable!("unsupported host state cannot be constructed")
        }
    }

    pub(crate) fn open_event_store_existing(
        &self,
        target: &Path,
        context: &crate::context::LiveContext,
        source_id: &str,
    ) -> Result<Option<HostEventStore>, HostFailure> {
        let mut store = self.open_event_store(target, context, source_id, false)?;
        if store.is_none() {
            return Ok(None);
        }
        let binding = crate::observability::SemanticEvent::for_context(
            context,
            source_id,
            "host-event-store-read-binding",
            0,
            0,
            "observe.store",
            "unknown",
        )
        .map_err(|_| HostFailure::Invalid)?;
        let checkpoints = self.inner.event_checkpoints(
            target,
            binding.context_id(),
            binding.candidate_id(),
            binding.source_id(),
        )?;
        if checkpoints.is_empty() {
            return Err(HostFailure::Invalid);
        }
        let mut expected = Vec::with_capacity(checkpoints.len());
        for checkpoint in &checkpoints {
            self.authenticate_checkpoint_for_history(target, checkpoint)?;
            expected.push(ExpectedTerminalEvent {
                event: terminal_semantic_event(target, context, checkpoint)?,
                required: checkpoint.state() == "terminal-event-joined",
            });
        }
        store.as_mut().ok_or(HostFailure::Invalid)?.expected = expected;
        Ok(store)
    }

    pub(crate) fn append_terminal_event(
        &self,
        target: &Path,
        context: &crate::context::LiveContext,
        checkpoint: &supported::ContinuationCheckpoint,
    ) -> Result<bool, HostFailure> {
        self.authenticate_checkpoint(target, checkpoint, false)?;
        let event = terminal_semantic_event(target, context, checkpoint)?;
        let mut store = self
            .open_event_store(target, context, "successor-runtime", true)?
            .ok_or(HostFailure::Invalid)?;
        store.expected = vec![ExpectedTerminalEvent {
            event: event.clone(),
            required: true,
        }];
        #[cfg(target_vendor = "apple")]
        {
            let appended = store
                .inner
                .store
                .append(&event)
                .map_err(|_| HostFailure::Invalid)?;
            self.verify_appended_terminal_event(target, context, checkpoint)?;
            Ok(appended)
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (target, context, checkpoint);
            unreachable!("unsupported host state cannot be constructed")
        }
    }

    fn verify_appended_terminal_event(
        &self,
        target: &Path,
        context: &crate::context::LiveContext,
        checkpoint: &supported::ContinuationCheckpoint,
    ) -> Result<(), HostFailure> {
        self.authenticate_checkpoint(target, checkpoint, false)?;
        self.verify()?;
        let mut store = self
            .open_event_store_existing(target, context, "successor-runtime")?
            .ok_or(HostFailure::Invalid)?;
        let expected_id = checkpoint.event_id();
        let expected = store
            .expected
            .iter_mut()
            .find(|item| item.event.event_id() == expected_id)
            .ok_or(HostFailure::Invalid)?;
        expected.required = true;
        store.validate_all().map_err(|_| HostFailure::Invalid)?;
        self.verify_event_store(&store)
    }

    pub(crate) fn verify_joined_terminal_event(
        &self,
        target: &Path,
        context: &crate::context::LiveContext,
        checkpoint: &supported::ContinuationCheckpoint,
    ) -> Result<(), HostFailure> {
        if checkpoint.state() != "terminal-event-joined"
            || !checkpoint.has_current_event_authority()
        {
            return Err(HostFailure::Invalid);
        }
        self.authenticate_checkpoint_for_history(target, checkpoint)?;
        self.verify()?;
        let mut store = self
            .open_event_store_existing(target, context, "successor-runtime")?
            .ok_or(HostFailure::Invalid)?;
        let expected = store
            .expected
            .iter_mut()
            .find(|item| item.event.event_id() == checkpoint.event_id())
            .ok_or(HostFailure::Invalid)?;
        expected.required = true;
        store.validate_all().map_err(|_| HostFailure::Invalid)?;
        self.verify_event_store(&store)
    }

    pub(crate) fn verify_event_store(&self, store: &HostEventStore) -> Result<(), HostFailure> {
        #[cfg(target_vendor = "apple")]
        {
            self.verify()?;
            self.inner.verify_event_store(&store.inner)?;
            store.validate_all().map_err(|_| HostFailure::Invalid)
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            unreachable!("unsupported host state cannot be constructed")
        }
    }

    pub(crate) fn event_store_absent(
        &self,
        target: &Path,
        context_id: &str,
        candidate_id: &str,
        source_id: &str,
    ) -> Result<bool, HostFailure> {
        #[cfg(target_vendor = "apple")]
        {
            self.verify()?;
            self.inner
                .event_store_absent(target, context_id, candidate_id, source_id)
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (target, context_id, candidate_id, source_id);
            unreachable!("unsupported host state cannot be constructed")
        }
    }

    pub(crate) fn open_or_bootstrap(home: &Path, target: &Path) -> Result<Self, HostFailure> {
        #[cfg(target_vendor = "apple")]
        {
            let state = Self {
                inner: supported::HostState::open_or_bootstrap(home, target)?,
            };
            state.inner.verify()?;
            state.verify_event_authority()?;
            Ok(state)
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (home, target);
            Err(HostFailure::Unsupported)
        }
    }

    pub(crate) fn issue_custody_capability(&self) -> Result<RoutineCustodyCapability, HostFailure> {
        #[cfg(target_vendor = "apple")]
        {
            Ok(RoutineCustodyCapability::issue_from_host(
                HostCustodyIssuance::from_host_state(&self.inner)?,
            ))
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            unreachable!("unsupported host state cannot be constructed")
        }
    }

    pub(crate) fn verify(&self) -> Result<(), HostFailure> {
        #[cfg(target_vendor = "apple")]
        {
            self.inner.verify()?;
            self.verify_event_authority()
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            unreachable!("unsupported host state cannot be constructed")
        }
    }

    pub(crate) fn verify_read_scope(&self) -> Result<(), HostFailure> {
        #[cfg(target_vendor = "apple")]
        {
            self.inner.verify_structure()
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            unreachable!("unsupported host state cannot be constructed")
        }
    }

    pub(crate) fn authenticate_checkpoint(
        &self,
        target: &Path,
        checkpoint: &supported::ContinuationCheckpoint,
        allow_stale_head: bool,
    ) -> Result<(), HostFailure> {
        if checkpoint.target() != target.to_str().ok_or(HostFailure::Invalid)? {
            return Err(HostFailure::Invalid);
        }
        self.authenticate_public_state(
            target,
            checkpoint.context_id(),
            checkpoint.candidate_id(),
            checkpoint.plan_id(),
            checkpoint.snapshot_id(),
            checkpoint.continuation(),
            checkpoint.recovery_marker(),
            checkpoint.predecessor_continuations(),
            checkpoint.attempt_grant(),
            checkpoint.ledger_head(),
            checkpoint.state(),
            checkpoint
                .terminal_outcome()
                .map(|outcome| outcome.as_str()),
            allow_stale_head,
        )
    }

    pub(crate) fn authenticate_checkpoint_for_history(
        &self,
        target: &Path,
        checkpoint: &supported::ContinuationCheckpoint,
    ) -> Result<(), HostFailure> {
        if self
            .authenticate_checkpoint(target, checkpoint, false)
            .is_ok()
        {
            return Ok(());
        }
        let settled = checkpoint.is_terminal()
            && checkpoint.terminal_outcome().is_some_and(|outcome| {
                matches!(
                    outcome,
                    crate::routine_work::RoutineTerminalOutcome::Complete
                        | crate::routine_work::RoutineTerminalOutcome::Failed
                        | crate::routine_work::RoutineTerminalOutcome::Cancelled
                        | crate::routine_work::RoutineTerminalOutcome::Incomplete
                )
            });
        if !settled {
            return Err(HostFailure::Invalid);
        }
        self.authenticate_checkpoint(target, checkpoint, true)
    }

    fn verify_event_authority(&self) -> Result<(), HostFailure> {
        #[cfg(target_vendor = "apple")]
        {
            for checkpoint in self.inner.all_event_checkpoints()? {
                let target = Path::new(checkpoint.target());
                self.authenticate_checkpoint_for_history(target, &checkpoint)?;
            }
            Ok(())
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            unreachable!("unsupported host state cannot be constructed")
        }
    }

    pub(crate) fn authenticate_public_state(
        &self,
        target: &Path,
        context_id: &str,
        candidate_id: &str,
        plan_id: &str,
        snapshot_id: &str,
        continuation: &str,
        recovery_marker: &str,
        predecessor_continuations: &[String],
        attempt_grant: &str,
        authenticated_ledger_head: &str,
        state: &str,
        terminal_outcome: Option<&str>,
        allow_stale_head: bool,
    ) -> Result<(), HostFailure> {
        #[cfg(target_vendor = "apple")]
        {
            crate::routine_work::authenticate_public_routine_checkpoint(
                self.issue_custody_capability()?,
                target,
                context_id,
                candidate_id,
                plan_id,
                snapshot_id,
                continuation,
                recovery_marker,
                predecessor_continuations,
                attempt_grant,
                authenticated_ledger_head,
                state,
                terminal_outcome,
                allow_stale_head,
            )
            .map_err(|_| HostFailure::Invalid)
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (
                target,
                context_id,
                candidate_id,
                plan_id,
                snapshot_id,
                continuation,
                recovery_marker,
                predecessor_continuations,
                attempt_grant,
                authenticated_ledger_head,
                state,
                terminal_outcome,
                allow_stale_head,
            );
            unreachable!("unsupported host state cannot be constructed")
        }
    }

    pub(crate) fn record_reserved_checkpoint(
        &self,
        request: ReservedCheckpoint<'_>,
    ) -> Result<(), HostFailure> {
        #[cfg(target_vendor = "apple")]
        {
            self.inner.record_reserved_checkpoint(request)
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = request;
            unreachable!("unsupported host state cannot be constructed")
        }
    }

    pub(crate) fn exact_checkpoint(
        &self,
        binding: CheckpointBinding<'_>,
        continuation: Option<&str>,
    ) -> Result<Option<supported::ContinuationCheckpoint>, HostFailure> {
        #[cfg(target_vendor = "apple")]
        {
            self.inner.exact_checkpoint(binding, continuation)
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (binding, continuation);
            unreachable!("unsupported host state cannot be constructed")
        }
    }

    pub(crate) fn exact_checkpoint_scoped(
        &self,
        binding: CheckpointBinding<'_>,
        continuation: Option<&str>,
    ) -> Result<Option<supported::ContinuationCheckpoint>, HostFailure> {
        #[cfg(target_vendor = "apple")]
        {
            self.inner.exact_checkpoint_scoped(binding, continuation)
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (binding, continuation);
            unreachable!("unsupported host state cannot be constructed")
        }
    }

    pub(crate) fn exact_checkpoint_or_legacy(
        &self,
        binding: CheckpointBinding<'_>,
    ) -> Result<Option<supported::ContinuationCheckpoint>, HostFailure> {
        match self.exact_checkpoint(binding, None)? {
            Some(checkpoint) => Ok(Some(checkpoint)),
            None if !binding.execution_id().is_empty() => {
                self.exact_checkpoint(binding.without_execution_id(), None)
            }
            None => Ok(None),
        }
    }

    pub(crate) fn resolve_checkpoint(
        &self,
        binding: CheckpointBinding<'_>,
        continuation: &str,
    ) -> Result<Option<supported::ContinuationResolution>, HostFailure> {
        #[cfg(target_vendor = "apple")]
        {
            match self.inner.resolve_checkpoint(binding, continuation)? {
                Some(resolution) => Ok(Some(resolution)),
                None if !binding.execution_id().is_empty() => self
                    .inner
                    .resolve_checkpoint(binding.without_execution_id(), continuation),
                None => Ok(None),
            }
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (binding, continuation);
            unreachable!("unsupported host state cannot be constructed")
        }
    }

    pub(crate) fn remove_alias(
        &self,
        binding: CheckpointBinding<'_>,
        alias: &supported::ContinuationCheckpoint,
    ) -> Result<(), HostFailure> {
        #[cfg(target_vendor = "apple")]
        {
            self.inner.remove_alias(binding, alias)
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (binding, alias);
            unreachable!("unsupported host state cannot be constructed")
        }
    }

    pub(crate) fn remove_alias_if_present(
        &self,
        binding: CheckpointBinding<'_>,
        alias: Option<&supported::ContinuationCheckpoint>,
    ) -> Result<(), HostFailure> {
        if let Some(alias) = alias {
            self.remove_alias(binding, alias)?;
        }
        Ok(())
    }

    pub(crate) fn record_terminal_checkpoint(
        &self,
        request: TerminalCheckpoint<'_>,
    ) -> Result<(), HostFailure> {
        #[cfg(target_vendor = "apple")]
        {
            self.inner.record_terminal_checkpoint(request)
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = request;
            unreachable!("unsupported host state cannot be constructed")
        }
    }

    pub(crate) fn mark_event_joined(
        &self,
        target: &Path,
        context: &crate::context::LiveContext,
        checkpoint: &supported::ContinuationCheckpoint,
    ) -> Result<(), HostFailure> {
        #[cfg(target_vendor = "apple")]
        {
            self.verify_appended_terminal_event(target, context, checkpoint)?;
            self.inner.mark_event_joined(checkpoint)
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (target, context, checkpoint);
            unreachable!("unsupported host state cannot be constructed")
        }
    }

    pub(crate) fn mark_checkpoint_reconciled(
        &self,
        checkpoint: &supported::ContinuationCheckpoint,
        authenticated_ledger_head: String,
    ) -> Result<(), HostFailure> {
        #[cfg(target_vendor = "apple")]
        {
            self.inner
                .mark_checkpoint_reconciled(checkpoint, authenticated_ledger_head)
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (checkpoint, authenticated_ledger_head);
            unreachable!("unsupported host state cannot be constructed")
        }
    }

    pub(crate) fn mark_checkpoint_ambiguous(
        &self,
        checkpoint: &supported::ContinuationCheckpoint,
    ) -> Result<(), HostFailure> {
        #[cfg(target_vendor = "apple")]
        {
            self.inner.mark_checkpoint_ambiguous(checkpoint)
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = checkpoint;
            unreachable!("unsupported host state cannot be constructed")
        }
    }
}

fn terminal_semantic_event(
    target: &Path,
    context: &crate::context::LiveContext,
    checkpoint: &supported::ContinuationCheckpoint,
) -> Result<crate::observability::SemanticEvent, HostFailure> {
    if context.context_id() != checkpoint.context_id()
        || crate::observability::SemanticEvent::for_context(
            context,
            "successor-runtime",
            "migration-context-check",
            0,
            0,
            "check.routine.terminal",
            "unknown",
        )
        .map_err(|_| HostFailure::Invalid)?
        .candidate_id()
            != checkpoint.candidate_id()
    {
        return Err(HostFailure::Invalid);
    }
    terminal_semantic_event_from_checkpoint(target, checkpoint)
}

pub(crate) fn terminal_semantic_event_from_checkpoint(
    target: &Path,
    checkpoint: &supported::ContinuationCheckpoint,
) -> Result<crate::observability::SemanticEvent, HostFailure> {
    if !checkpoint.is_terminal()
        || !checkpoint.has_current_event_authority()
        || checkpoint.target() != target.to_str().ok_or(HostFailure::Invalid)?
    {
        return Err(HostFailure::Invalid);
    }
    let mut event =
        crate::observability::SemanticEvent::new(crate::observability::SemanticEventInput {
            context_id: checkpoint.context_id().to_owned(),
            candidate_id: checkpoint.candidate_id().to_owned(),
            source_id: "successor-runtime".to_owned(),
            event_id: checkpoint.event_id().to_owned(),
            observed_at_unix_ms: checkpoint.event_observed_at_unix_ms(),
            sequence: checkpoint.event_sequence(),
            operation: "check.routine.terminal".to_owned(),
            outcome: checkpoint.event_status().to_owned(),
        })
        .map_err(|_| HostFailure::Invalid)?;
    if let Some(parent_event_id) = checkpoint.event_parent_id() {
        event
            .set_parent(parent_event_id)
            .map_err(|_| HostFailure::Invalid)?;
    }
    let target_binding = format!("sha256:{:x}", Sha256::digest(target.as_os_str().as_bytes()));
    for (key, value) in [
        ("target_binding", target_binding.as_str()),
        ("plan_id", checkpoint.plan_id()),
        ("snapshot_id", checkpoint.snapshot_id()),
        ("execution_id", checkpoint.execution_id()),
        ("continuation_id", checkpoint.continuation()),
        ("terminal_ledger_head", checkpoint.ledger_head()),
        ("routine_transition", checkpoint.event_transition()),
        (
            "routine_terminal_outcome",
            checkpoint
                .terminal_outcome()
                .ok_or(HostFailure::Invalid)?
                .as_str(),
        ),
    ] {
        event
            .add_public_attribute(key, value)
            .map_err(|_| HostFailure::Invalid)?;
    }
    if let Some(binding) = checkpoint.finding_binding() {
        event
            .add_finding_ref(&binding.finding_id)
            .and_then(|_| event.add_repair_ref(&binding.repair_id))
            .map_err(|_| HostFailure::Invalid)?;
    }
    Ok(event)
}

fn validate_expected_events(
    events: &[crate::observability::SemanticEvent],
    expected: &[ExpectedTerminalEvent],
) -> Result<(), String> {
    let has_only_expected = events
        .iter()
        .all(|event| expected.iter().any(|item| item.event == *event));
    let has_valid_cardinality = expected.iter().all(|item| {
        let count = events.iter().filter(|event| **event == item.event).count();
        count == usize::from(item.required) || (!item.required && count <= 1)
    });
    if has_only_expected && has_valid_cardinality {
        Ok(())
    } else {
        Err("observe-store-corrupt:unauthenticated-terminal-event".to_owned())
    }
}

#[cfg(test)]
mod event_projection_tests {
    use super::{ExpectedTerminalEvent, validate_expected_events};
    use crate::observability::{SemanticEvent, SemanticEventInput};

    fn event() -> SemanticEvent {
        let mut event = SemanticEvent::new(SemanticEventInput {
            context_id: "sha256:context".to_owned(),
            candidate_id: "sha256:candidate".to_owned(),
            source_id: "successor-runtime".to_owned(),
            event_id: "routine-terminal-test".to_owned(),
            observed_at_unix_ms: 1,
            sequence: 1,
            operation: "check.routine.terminal".to_owned(),
            outcome: "pass".to_owned(),
        })
        .unwrap();
        event
            .add_public_attribute("plan_id", "sha256:plan")
            .unwrap();
        event.add_finding_ref("sha256:finding").unwrap();
        event.add_repair_ref("repair-test").unwrap();
        event
    }

    #[test]
    fn joined_projection_is_required_exactly_once_while_pending_may_be_absent() {
        let terminal = event();
        let joined = [ExpectedTerminalEvent {
            event: terminal.clone(),
            required: true,
        }];
        assert!(validate_expected_events(&[terminal.clone()], &joined).is_ok());
        assert!(validate_expected_events(&[], &joined).is_err());
        assert!(validate_expected_events(&[terminal.clone(), terminal.clone()], &joined).is_err());

        let pending = [ExpectedTerminalEvent {
            event: terminal,
            required: false,
        }];
        assert!(validate_expected_events(&[], &pending).is_ok());
    }

    #[test]
    fn extra_or_mutated_rows_never_become_authenticated_observations() {
        let terminal = event();
        let expected = [ExpectedTerminalEvent {
            event: terminal.clone(),
            required: true,
        }];

        let mut mutated_attribute = terminal.clone();
        mutated_attribute
            .add_public_attribute("snapshot_id", "sha256:forged")
            .unwrap();
        assert!(validate_expected_events(&[mutated_attribute], &expected).is_err());

        let mut mutated_reference = terminal.clone();
        mutated_reference.add_finding_ref("sha256:forged").unwrap();
        assert!(validate_expected_events(&[mutated_reference], &expected).is_err());

        let mut forged = terminal.clone();
        forged.set_parent("forged-parent").unwrap();
        assert!(validate_expected_events(&[terminal, forged], &expected).is_err());
    }
}
