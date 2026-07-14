impl Effects {
    fn restore_inner(&mut self, prior: &LifecycleState) -> Result<(), AdapterError> {
        if self.restored || prior != &self.before {
            return Err(AdapterError::new(AdapterErrorId::RecoveryRefused));
        }
        self.require_logical_state()?;
        while let Some(mutation) = self.mutations.pop() {
            let surface = match mutation {
                Mutation::Installed {
                    surface,
                    transaction,
                } => {
                    rollback_install(transaction, &mut ScopedInstall::new(self.root.clone()))
                        .map_err(AdapterError::distribution)?;
                    surface
                }
                Mutation::Removed {
                    surface,
                    file,
                    previous,
                } => {
                    if !file
                        .apply(None, Some(&previous))
                        .map_err(AdapterError::distribution)?
                    {
                        return Err(AdapterError::new(AdapterErrorId::RecoveryRefused));
                    }
                    surface
                }
            };
            let prior_value = match surface {
                Surface::Installed => self.before.installed.clone(),
                Surface::Cache => self.before.cache.clone(),
            };
            self.set_surface(surface, prior_value);
            self.logical.recovery_required = true;
        }
        self.logical = prior.clone();
        self.require_logical_state()?;
        self.restored = true;
        Ok(())
    }
}

impl LifecycleEffectAdapter for Effects {
    fn execute(
        &mut self,
        effect: LifecycleEffect,
        expected_after: &LifecycleState,
    ) -> Result<(), String> {
        self.execution_attempted = true;
        let result = (|| {
            if self.restored
                || expected_after != &self.expected_after
                || self.effects.get(self.next_effect) != Some(&effect)
            {
                return Err(AdapterError::new(AdapterErrorId::EffectOrder));
            }
            let mutating = matches!(
                effect,
                LifecycleEffect::InstallPackage
                    | LifecycleEffect::RefreshCache
                    | LifecycleEffect::RestorePriorAuthority
                    | LifecycleEffect::RemoveInstalledPackage
                    | LifecycleEffect::RemoveCache
            );
            if mutating {
                self.require_logical_state()?;
            }
            self.execute_effect(effect)?;
            self.next_effect += 1;
            if self.next_effect == self.effects.len() {
                self.logical = self.expected_after.clone();
            }
            if mutating || self.next_effect == self.effects.len() {
                self.require_logical_state()?;
            }
            Ok(())
        })();
        result.map_err(|failure| failure.to_string())
    }

    fn restore(&mut self, prior: &LifecycleState) -> Result<(), String> {
        self.restore_inner(prior)
            .map_err(|failure| failure.to_string())
    }

    fn observe_state(&self) -> Result<LifecycleState, String> {
        self.observed_state().map_err(|failure| failure.to_string())
    }
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
