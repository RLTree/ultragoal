#[cfg(test)]
use crate::plugin_product::lifecycle::LifecycleEffect;
use crate::plugin_product::lifecycle::LifecycleState;

pub(super) struct ObservedRecoveryAdapter {
    pub(super) observed: LifecycleState,
}

impl crate::plugin_product::lifecycle::model::LifecycleEffectAdapter for ObservedRecoveryAdapter {
    #[cfg(test)]
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

    #[cfg(test)]
    fn observe_state(&self) -> Result<LifecycleState, String> {
        Ok(self.observed.clone())
    }
}
