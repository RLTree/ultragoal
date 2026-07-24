use super::HostCustodyIssuance;
use super::*;
use crate::routine_work::RoutineCustodyCapability;
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

impl HostState {
    pub(crate) fn open_existing(home: &Path) -> Result<Self, HostFailure> {
        #[cfg(target_vendor = "apple")]
        {
            supported::HostState::open_existing(home).map(|inner| Self { inner })
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = home;
            Err(HostFailure::Unsupported)
        }
    }

    pub(crate) fn open_or_bootstrap(home: &Path, target: &Path) -> Result<Self, HostFailure> {
        #[cfg(target_vendor = "apple")]
        {
            supported::HostState::open_or_bootstrap(home, target).map(|inner| Self { inner })
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (home, target);
            Err(HostFailure::Unsupported)
        }
    }

    pub(crate) fn issue_custody_capability(&self) -> RoutineCustodyCapability {
        #[cfg(target_vendor = "apple")]
        {
            RoutineCustodyCapability::issue_from_host(HostCustodyIssuance::new(
                self.inner.authority.path.clone(),
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
            self.inner.verify()
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
            // A completed attempt is a read-only replay: its exact terminal
            // binding remains valid when another, independently bound attempt
            // advances the shared ledger. Reserved recovery stays exact-head
            // fenced unless this is the one-shot reconciled alias path.
            allow_stale_head || checkpoint.is_complete(),
        )
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
                self.issue_custody_capability(),
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
        checkpoint: &supported::ContinuationCheckpoint,
    ) -> Result<(), HostFailure> {
        #[cfg(target_vendor = "apple")]
        {
            self.inner.mark_event_joined(checkpoint)
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = checkpoint;
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
