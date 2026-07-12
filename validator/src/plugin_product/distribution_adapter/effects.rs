use super::binding::BoundPackage;
use super::error::{AdapterError, AdapterErrorId};
use super::mutation::{Mutation, Surface};
use super::observation::{CACHE_PATH, INSTALLED_PATH, observe, verify_surface};
use crate::distribution::{
    ConfinedRoot, ExpectedPrior, InstallPlan, InstallScope, ScopedFile, ScopedInstall, install,
    rollback_install,
};
use crate::plugin_product::lifecycle::{LifecycleEffect, LifecycleEffectAdapter, LifecycleState};
use sha2::{Digest, Sha256};

pub(super) struct Effects {
    root: ConfinedRoot,
    package: BoundPackage,
    before: LifecycleState,
    expected_after: LifecycleState,
    effects: Vec<LifecycleEffect>,
    next_effect: usize,
    logical: LifecycleState,
    mutations: Vec<Mutation>,
    execution_attempted: bool,
    restored: bool,
}

impl Effects {
    pub(super) fn bind(
        root: ConfinedRoot,
        package: BoundPackage,
        before: LifecycleState,
        expected_after: LifecycleState,
        effects: Vec<LifecycleEffect>,
    ) -> Result<Self, AdapterError> {
        let result = Self {
            root,
            package,
            logical: before.clone(),
            before,
            expected_after,
            effects,
            next_effect: 0,
            mutations: Vec::new(),
            execution_attempted: false,
            restored: false,
        };
        result.require_logical_state()?;
        Ok(result)
    }

    pub(super) fn observed_state(&self) -> Result<LifecycleState, AdapterError> {
        observe(
            &self.root,
            &self.logical,
            &self.before,
            &self.expected_after,
            &self.package,
        )
    }

    pub(super) fn completed_effects(&self) -> &[LifecycleEffect] {
        &self.effects[..self.next_effect]
    }

    pub(super) fn mutation_count(&self) -> usize {
        self.mutations.len()
    }

    pub(super) fn execution_attempted(&self) -> bool {
        self.execution_attempted
    }

    fn require_logical_state(&self) -> Result<(), AdapterError> {
        if self.observed_state()? != self.logical {
            return Err(AdapterError::new(AdapterErrorId::PhysicalStateMismatch));
        }
        Ok(())
    }

    fn install_surface(&mut self, surface: Surface) -> Result<(), AdapterError> {
        let (path, current, replacement) = match surface {
            Surface::Installed => (
                INSTALLED_PATH,
                self.logical.installed.as_ref(),
                self.expected_after.installed.clone(),
            ),
            Surface::Cache => (
                CACHE_PATH,
                self.logical.cache.as_ref(),
                self.expected_after.cache.clone(),
            ),
        };
        if replacement.as_ref() != Some(self.package.authority()) {
            return Err(AdapterError::new(AdapterErrorId::InvalidLifecycleBinding));
        }
        let expected_prior = current.map_or(ExpectedPrior::Absent, |row| {
            ExpectedPrior::ExactDigest(row.package_sha256.clone())
        });
        let plan = InstallPlan::new(
            self.package.snapshot().context_id().to_owned(),
            self.package.snapshot().candidate_id().to_owned(),
            InstallScope::PersonalFixture,
            path.to_owned(),
            self.package.snapshot().package_sha256().to_owned(),
            expected_prior,
        )
        .map_err(AdapterError::distribution)?;
        let transaction = install(
            &plan,
            self.package.snapshot(),
            &mut ScopedInstall::new(self.root.clone()),
        )
        .map_err(AdapterError::distribution)?;
        self.mutations.push(Mutation::Installed {
            surface,
            transaction,
        });
        self.set_surface(surface, replacement);
        self.logical.generation = self.expected_after.generation;
        self.logical.recovery_required = true;
        Ok(())
    }

    fn remove_surface(&mut self, surface: Surface) -> Result<(), AdapterError> {
        let (path, current) = match surface {
            Surface::Installed => (INSTALLED_PATH, self.logical.installed.as_ref()),
            Surface::Cache => (CACHE_PATH, self.logical.cache.as_ref()),
        };
        let Some(current) = current else {
            self.set_surface(surface, None);
            return Ok(());
        };
        let file = ScopedFile::new(self.root.clone(), path).map_err(AdapterError::distribution)?;
        let previous = file
            .inspect(65 * 1024 * 1024)
            .map_err(AdapterError::distribution)?
            .ok_or_else(|| AdapterError::new(AdapterErrorId::PhysicalStateMismatch))?;
        if digest(&previous) != current.package_sha256
            || !file
                .apply(Some(&current.package_sha256), None)
                .map_err(AdapterError::distribution)?
        {
            return Err(AdapterError::new(AdapterErrorId::PhysicalStateMismatch));
        }
        self.mutations.push(Mutation::Removed {
            surface,
            file,
            previous,
        });
        self.set_surface(surface, None);
        self.logical.generation = self.expected_after.generation;
        self.logical.recovery_required = true;
        Ok(())
    }

    fn set_surface(
        &mut self,
        surface: Surface,
        value: Option<crate::plugin_product::lifecycle::PackageAuthority>,
    ) {
        match surface {
            Surface::Installed => self.logical.installed = value,
            Surface::Cache => self.logical.cache = value,
        }
    }

    fn execute_effect(&mut self, effect: LifecycleEffect) -> Result<(), AdapterError> {
        match effect {
            LifecycleEffect::InstallPackage => self.install_surface(Surface::Installed),
            LifecycleEffect::RefreshCache => self.install_surface(Surface::Cache),
            LifecycleEffect::RestorePriorAuthority => {
                self.install_surface(Surface::Installed)?;
                self.install_surface(Surface::Cache)
            }
            LifecycleEffect::RemoveInstalledPackage => self.remove_surface(Surface::Installed),
            LifecycleEffect::RemoveCache => self.remove_surface(Surface::Cache),
            LifecycleEffect::VerifyInstalledBytes => verify_surface(
                &self.root,
                INSTALLED_PATH,
                self.expected_after.installed.as_ref(),
                &self.package,
            ),
            LifecycleEffect::VerifyTeardown => {
                verify_surface(&self.root, INSTALLED_PATH, None, &self.package)?;
                verify_surface(&self.root, CACHE_PATH, None, &self.package)
            }
            LifecycleEffect::ProbeRuntime => {
                // This adapter's probe is deliberately limited to a second,
                // zero-write distribution identity observation. It does not
                // claim that a Codex runtime loaded or executed the plugin.
                verify_surface(
                    &self.root,
                    INSTALLED_PATH,
                    self.expected_after.installed.as_ref(),
                    &self.package,
                )?;
                verify_surface(
                    &self.root,
                    CACHE_PATH,
                    self.expected_after.cache.as_ref(),
                    &self.package,
                )
            }
        }
    }

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
