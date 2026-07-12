use super::model::{EffectClass, EffectGrant, MAX_COLLECTION, bounded, validate_identifier};
use super::scope::{effect_contains, path_is_within, symbol_in_prefix};
use super::{CanonicalPath, OrchestrationError, OwnedScope};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ScopePolicy {
    pub allowed_read_paths: BTreeSet<CanonicalPath>,
    pub allowed_paths: BTreeSet<CanonicalPath>,
    pub allowed_semantic_prefixes: BTreeSet<String>,
    pub allowed_generated_outputs: BTreeSet<CanonicalPath>,
    pub allowed_fixtures: BTreeSet<CanonicalPath>,
    pub allowed_effects: BTreeSet<EffectGrant>,
    pub root_only_paths: BTreeSet<CanonicalPath>,
    pub root_only_semantic_prefixes: BTreeSet<String>,
    pub root_only_effect_classes: BTreeSet<EffectClass>,
}

impl ScopePolicy {
    pub fn validate(&self) -> Result<(), OrchestrationError> {
        bounded(&self.allowed_read_paths)?;
        bounded(&self.allowed_paths)?;
        bounded(&self.allowed_semantic_prefixes)?;
        bounded(&self.allowed_generated_outputs)?;
        bounded(&self.allowed_fixtures)?;
        bounded(&self.allowed_effects)?;
        bounded(&self.root_only_paths)?;
        bounded(&self.root_only_semantic_prefixes)?;
        bounded(&self.root_only_effect_classes)?;
        for prefix in self
            .allowed_semantic_prefixes
            .iter()
            .chain(self.root_only_semantic_prefixes.iter())
        {
            validate_identifier(prefix)?;
        }
        for effect in &self.allowed_effects {
            effect.validate()?;
        }
        Ok(())
    }

    pub fn validate_worker_reads(
        &self,
        reads: &BTreeSet<CanonicalPath>,
    ) -> Result<(), OrchestrationError> {
        if reads.len() > MAX_COLLECTION {
            return Err(OrchestrationError::ResourceLimit);
        }
        if reads.iter().any(is_host_protected) {
            return Err(OrchestrationError::RootOnlyScope);
        }
        if reads
            .iter()
            .any(|path| !path_allowed(path, &self.allowed_read_paths))
        {
            return Err(OrchestrationError::UnknownScope);
        }
        Ok(())
    }

    pub fn validate_worker_scope(&self, scope: &OwnedScope) -> Result<(), OrchestrationError> {
        scope.validate()?;
        if self.requests_root_authority(scope) {
            return Err(OrchestrationError::RootOnlyScope);
        }
        if scope
            .paths
            .iter()
            .any(|path| !path_allowed(path, &self.allowed_paths))
            || scope.semantic_symbols.iter().any(|symbol| {
                !self
                    .allowed_semantic_prefixes
                    .iter()
                    .any(|prefix| symbol_in_prefix(symbol, prefix))
            })
            || scope
                .generated_outputs
                .iter()
                .any(|path| !path_allowed(path, &self.allowed_generated_outputs))
            || scope
                .fixtures
                .iter()
                .any(|path| !path_allowed(path, &self.allowed_fixtures))
            || scope.effects.iter().any(|requested| {
                !self
                    .allowed_effects
                    .iter()
                    .any(|allowed| effect_contains(allowed, requested))
            })
        {
            return Err(OrchestrationError::UnknownScope);
        }
        Ok(())
    }

    fn requests_root_authority(&self, scope: &OwnedScope) -> bool {
        scope
            .paths
            .iter()
            .chain(scope.generated_outputs.iter())
            .chain(scope.fixtures.iter())
            .any(|path| {
                is_host_protected(path)
                    || self.root_only_paths.iter().any(|root| root.overlaps(path))
            })
            || scope.semantic_symbols.iter().any(|symbol| {
                self.root_only_semantic_prefixes
                    .iter()
                    .any(|root| symbol_in_prefix(symbol, root) || symbol_in_prefix(root, symbol))
            })
            || scope.effects.iter().any(|effect| {
                effect.class == EffectClass::RootAuthority
                    || self.root_only_effect_classes.contains(&effect.class)
            })
    }
}

fn is_host_protected(path: &CanonicalPath) -> bool {
    const HOST_ROOTS: &[&str] = &[".git", ".codex", ".agents", ".codex-worktree"];
    path.as_str().split('/').next().is_some_and(|component| {
        HOST_ROOTS
            .iter()
            .any(|protected| component.eq_ignore_ascii_case(protected))
    })
}

fn path_allowed(path: &CanonicalPath, allowed: &BTreeSet<CanonicalPath>) -> bool {
    allowed.iter().any(|root| path_is_within(path, root))
}
