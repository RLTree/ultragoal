use super::super::lifecycle::DescriptorExecutionHandoff;
use super::{HostEffectExecutionReceipt, HostEffectExecutorFailure};

/// Keeps the authority-bearing handoff alive until the root transaction has
/// observed and durably settled the execution result. An executor failure is
/// returned with the same custody, so unresolved recovery cannot accidentally
/// trigger staged-executable cleanup.
pub(crate) struct RetainedExecutionOutcome {
    handoff: DescriptorExecutionHandoff,
    result: Result<HostEffectExecutionReceipt, HostEffectExecutorFailure>,
}

impl RetainedExecutionOutcome {
    pub(super) fn new(
        handoff: DescriptorExecutionHandoff,
        result: Result<HostEffectExecutionReceipt, HostEffectExecutorFailure>,
    ) -> Self {
        Self { handoff, result }
    }

    pub(in crate::distribution::host_effect) fn into_parts(
        self,
    ) -> (
        DescriptorExecutionHandoff,
        Result<HostEffectExecutionReceipt, HostEffectExecutorFailure>,
    ) {
        (self.handoff, self.result)
    }

    #[cfg(test)]
    pub(crate) fn unwrap_err(self) -> HostEffectExecutorFailure {
        self.result.expect_err("executor handoff should fail")
    }
}
