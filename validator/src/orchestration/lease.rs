use super::model::{MAX_COLLECTION, validate_actor_identifier, validate_identifier};
use super::{Actor, Binding, OrchestrationError, OwnedScope, Principal, SafetyClass, ScopePolicy};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const MAX_RETRIES: u8 = 8;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrerequisiteEvidence {
    pub dependency_nodes: BTreeMap<String, String>,
    pub required_tools: BTreeMap<String, String>,
    pub prerequisites: BTreeMap<String, String>,
}

impl PrerequisiteEvidence {
    fn validate(&self) -> Result<(), OrchestrationError> {
        for values in [
            &self.dependency_nodes,
            &self.required_tools,
            &self.prerequisites,
        ] {
            if values.len() > MAX_COLLECTION {
                return Err(OrchestrationError::ResourceLimit);
            }
            for (key, digest) in values {
                validate_identifier(key)?;
                super::model::validate_digest(digest)?;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeaseSpec {
    pub lease_id: String,
    pub run_id: String,
    pub node_id: String,
    pub principal: Principal,
    pub owner: Actor,
    pub binding: Binding,
    pub safety_class: SafetyClass,
    pub read_paths: BTreeSet<super::CanonicalPath>,
    pub owned_scope: OwnedScope,
    pub prerequisite_evidence: PrerequisiteEvidence,
    pub issued_tick: u64,
    pub heartbeat_deadline_tick: u64,
    pub max_retries: u8,
}

impl LeaseSpec {
    pub(crate) fn validate_read_write_disjoint(&self) -> Result<(), OrchestrationError> {
        if self.owned_scope.overlaps_read_paths(&self.read_paths) {
            return Err(OrchestrationError::InvalidLease);
        }
        Ok(())
    }

    pub fn validate(&self, policy: &ScopePolicy) -> Result<(), OrchestrationError> {
        validate_identifier(&self.lease_id)?;
        validate_identifier(&self.run_id)?;
        validate_identifier(&self.node_id)?;
        validate_actor_identifier(self.owner.as_str())?;
        self.binding.validate()?;
        self.owned_scope.validate()?;
        self.prerequisite_evidence.validate()?;
        if self.read_paths.len() > MAX_COLLECTION
            || self.heartbeat_deadline_tick <= self.issued_tick
            || self.max_retries > MAX_RETRIES
        {
            return Err(OrchestrationError::InvalidLease);
        }
        self.validate_read_write_disjoint()?;
        if self.principal == Principal::Worker {
            if self.safety_class == SafetyClass::RootSerialized {
                return Err(OrchestrationError::RootOnlyScope);
            }
            policy.validate_worker_scope(&self.owned_scope)?;
            policy.validate_worker_reads(&self.read_paths)?;
        } else if self.safety_class != SafetyClass::RootSerialized {
            return Err(OrchestrationError::InvalidLease);
        }
        match self.safety_class {
            SafetyClass::ReadOnly if !self.owned_scope.is_empty() => {
                return Err(OrchestrationError::InvalidLease);
            }
            SafetyClass::IsolatedWorkspaceWrite if self.owned_scope.is_empty() => {
                return Err(OrchestrationError::InvalidLease);
            }
            SafetyClass::ExternalBounded
                if self.owned_scope.effects.is_empty()
                    || self.owned_scope.effects.iter().any(|effect| {
                        matches!(
                            effect.class,
                            super::EffectClass::Destructive | super::EffectClass::RootAuthority
                        )
                    }) =>
            {
                return Err(OrchestrationError::InvalidLease);
            }
            _ => {}
        }
        Ok(())
    }

    pub fn conflicts(&self, other: &Self) -> bool {
        self.binding == other.binding
            && (self.owned_scope.conflicts(&other.owned_scope)
                || self.safety_class == SafetyClass::RootSerialized
                || other.safety_class == SafetyClass::RootSerialized)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LeaseRegistry {
    active: BTreeMap<String, LeaseSpec>,
}

impl LeaseRegistry {
    pub fn grant(
        &mut self,
        lease: LeaseSpec,
        policy: &ScopePolicy,
        binding: &Binding,
    ) -> Result<(), OrchestrationError> {
        lease.validate(policy)?;
        if &lease.binding != binding {
            return Err(OrchestrationError::StaleBinding);
        }
        if self.active.len() >= MAX_COLLECTION {
            return Err(OrchestrationError::ResourceLimit);
        }
        if self.active.contains_key(&lease.lease_id) {
            return Err(OrchestrationError::InvalidLease);
        }
        if self
            .active
            .values()
            .any(|active| active.node_id == lease.node_id)
        {
            return Err(OrchestrationError::LeaseConflict);
        }
        if self.active.values().any(|active| active.conflicts(&lease)) {
            return Err(OrchestrationError::LeaseConflict);
        }
        self.active.insert(lease.lease_id.clone(), lease);
        Ok(())
    }

    pub fn revoke(&mut self, lease_id: &str) -> Option<LeaseSpec> {
        self.active.remove(lease_id)
    }

    pub fn get(&self, lease_id: &str) -> Option<&LeaseSpec> {
        self.active.get(lease_id)
    }

    pub fn values(&self) -> impl Iterator<Item = &LeaseSpec> {
        self.active.values()
    }
}
