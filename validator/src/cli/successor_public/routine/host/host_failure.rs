use super::HostCustodyIssuance;
use super::*;
use crate::routine_work::RoutineCustodyCapability;
use crate::state::RoutineFindingBinding;
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
        finding_binding: Option<&RoutineFindingBinding>,
    ) -> Result<(), HostFailure> {
        #[cfg(target_vendor = "apple")]
        {
            self.inner.record_reserved_checkpoint(
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
            )
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
                attempt_grant,
                authenticated_ledger_head,
                finding_binding,
            );
            unreachable!("unsupported host state cannot be constructed")
        }
    }

    pub(crate) fn exact_checkpoint(
        &self,
        target: &Path,
        context_id: &str,
        candidate_id: &str,
        plan_id: &str,
        snapshot_id: &str,
        continuation: Option<&str>,
    ) -> Result<Option<supported::ContinuationCheckpoint>, HostFailure> {
        #[cfg(target_vendor = "apple")]
        {
            self.inner.exact_checkpoint(
                target,
                context_id,
                candidate_id,
                plan_id,
                snapshot_id,
                continuation,
            )
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
            );
            unreachable!("unsupported host state cannot be constructed")
        }
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
        finding_binding: Option<&RoutineFindingBinding>,
        terminal_outcome: crate::routine_work::RoutineTerminalOutcome,
    ) -> Result<(), HostFailure> {
        #[cfg(target_vendor = "apple")]
        {
            self.inner.record_terminal_checkpoint(
                target,
                context_id,
                candidate_id,
                plan_id,
                snapshot_id,
                continuation,
                attempt_grant,
                authenticated_ledger_head,
                finding_binding,
                terminal_outcome,
            )
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
                attempt_grant,
                authenticated_ledger_head,
                finding_binding,
                terminal_outcome,
            );
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
