use crate::plugin_product::lifecycle::{LifecycleEffect, LifecycleState};

#[cfg(not(test))]
use super::HostEffectRecoveryHandoff;

pub(super) struct ObservedRecoveryAdapter {
    pub(super) observed: LifecycleState,
}

impl crate::plugin_product::lifecycle::model::LifecycleEffectAdapter for ObservedRecoveryAdapter {
    fn execute(
        &mut self,
        _effect: LifecycleEffect,
        _expected_after: &LifecycleState,
    ) -> Result<(), String> {
        Err("recovery observation cannot execute a host effect".to_owned())
    }

    fn restore(&mut self, prior: &LifecycleState) -> Result<(), String> {
        (self.observed.installed == prior.installed && self.observed.cache == prior.cache)
            .then_some(())
            .ok_or_else(|| "observed host authority differs from recovery prior".to_owned())
    }

    fn observe_state(&self) -> Result<LifecycleState, String> {
        Ok(self.observed.clone())
    }
}

#[cfg(not(test))]
pub(super) fn finalize_recorded_recovery(
    handoff: super::lifecycle::DescriptorExecutionHandoff,
    custody: &mut crate::plugin_product::lifecycle::HostLifecycleCustody,
    recovery: &HostEffectRecoveryHandoff,
    cause: &'static str,
) -> Result<super::transaction::HostLifecycleTransactionResult, &'static str> {
    let disposition = match handoff.issue_recovery_disposition(custody, recovery) {
        Ok(disposition) => disposition,
        Err(_) => {
            handoff.retain_for_recovery();
            return Err("host lifecycle recovery disposition unavailable");
        }
    };
    match handoff.finalize_recovery(disposition, recovery) {
        Ok(()) => Err(cause),
        Err(_) => Err("host lifecycle recovery finalization failed"),
    }
}
