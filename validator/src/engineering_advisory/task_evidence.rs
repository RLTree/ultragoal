use super::AdvisoryError;
use crate::orchestration::{
    Binding, CanonicalPath, LeaseRegistry, LeaseSpec, PrerequisiteEvidence, Principal, SafetyClass,
    ScopePolicy, WorkPackage,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const TASK_EVIDENCE_NO_CLAIM: &str =
    "This advisory packet is proposal-only and cannot promote readiness, release, or completion.";
const ROOT_LEASE_REVOCATION: &str = "root_lease_revocation";

/// A read-only projection of the task contract. It is not a worker result.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskEvidencePacket {
    pub schema_version: String,
    pub binding: Binding,
    pub node_id: String,
    pub run_id: String,
    pub lease_id: String,
    pub worker: String,
    pub principal: Principal,
    pub safety_class: SafetyClass,
    pub dependencies: BTreeSet<String>,
    pub required_tools: BTreeSet<String>,
    pub read_paths: BTreeSet<CanonicalPath>,
    pub owned_scope: crate::orchestration::OwnedScope,
    pub prerequisites: BTreeSet<String>,
    pub prerequisite_evidence: PrerequisiteEvidence,
    pub outputs: BTreeSet<String>,
    pub required_verification: BTreeSet<String>,
    pub issued_tick: u64,
    pub heartbeat_deadline_tick: u64,
    pub max_retries: u8,
    pub cancellation_authority: String,
    pub claim_effect: String,
    pub no_claim_statement: String,
}

impl TaskEvidencePacket {
    pub fn from_work_package(
        package: &WorkPackage,
        lease: &LeaseSpec,
        registry: &LeaseRegistry,
        policy: &ScopePolicy,
        current_binding: &Binding,
    ) -> Result<Self, AdvisoryError> {
        package
            .validate()
            .map_err(|_| AdvisoryError::PermissionWidening)?;
        lease
            .validate(policy)
            .map_err(|_| AdvisoryError::PermissionWidening)?;
        if &lease.binding != current_binding {
            return Err(AdvisoryError::StaleBinding);
        }
        require_active_lease(registry, lease)?;
        if lease.node_id != package.node_id
            || lease.safety_class != package.safety_class
            || lease.read_paths != package.read_paths
            || !lease.owned_scope.is_subset_of(&package.owned_scope)
        {
            return Err(AdvisoryError::PermissionWidening);
        }
        let packet = Self {
            schema_version: "TaskEvidencePacket-v1".to_owned(),
            binding: lease.binding.clone(),
            node_id: package.node_id.clone(),
            run_id: lease.run_id.clone(),
            lease_id: lease.lease_id.clone(),
            worker: lease.owner.as_str().to_owned(),
            principal: lease.principal,
            safety_class: lease.safety_class,
            dependencies: package.dependencies.clone(),
            required_tools: package.required_tools.clone(),
            read_paths: lease.read_paths.clone(),
            owned_scope: lease.owned_scope.clone(),
            prerequisites: package.prerequisites.clone(),
            prerequisite_evidence: lease.prerequisite_evidence.clone(),
            outputs: package.outputs.clone(),
            required_verification: package.acceptance.clone(),
            issued_tick: lease.issued_tick,
            heartbeat_deadline_tick: lease.heartbeat_deadline_tick,
            max_retries: lease.max_retries,
            cancellation_authority: ROOT_LEASE_REVOCATION.to_owned(),
            claim_effect: package.claim_effect.clone(),
            no_claim_statement: TASK_EVIDENCE_NO_CLAIM.to_owned(),
        };
        packet.validate_against(package, lease, registry, policy, current_binding)?;
        Ok(packet)
    }

    pub fn validate_against(
        &self,
        package: &WorkPackage,
        lease: &LeaseSpec,
        registry: &LeaseRegistry,
        policy: &ScopePolicy,
        current_binding: &Binding,
    ) -> Result<(), AdvisoryError> {
        package
            .validate()
            .map_err(|_| AdvisoryError::PermissionWidening)?;
        lease
            .validate(policy)
            .map_err(|_| AdvisoryError::PermissionWidening)?;
        require_active_lease(registry, lease)?;
        if self.schema_version != "TaskEvidencePacket-v1"
            || self.no_claim_statement != TASK_EVIDENCE_NO_CLAIM
            || self.binding != lease.binding
            || &self.binding != current_binding
            || self.node_id != package.node_id
            || self.node_id != lease.node_id
            || self.run_id != lease.run_id
            || self.lease_id != lease.lease_id
            || self.worker != lease.owner.as_str()
            || self.principal != lease.principal
            || self.safety_class != lease.safety_class
            || self.dependencies != package.dependencies
            || self.required_tools != package.required_tools
            || self.prerequisites != package.prerequisites
            || self.prerequisite_evidence != lease.prerequisite_evidence
            || self.outputs != package.outputs
            || self.issued_tick != lease.issued_tick
            || self.heartbeat_deadline_tick != lease.heartbeat_deadline_tick
            || self.max_retries != lease.max_retries
            || self.cancellation_authority != ROOT_LEASE_REVOCATION
            || self.claim_effect != package.claim_effect
        {
            return Err(AdvisoryError::StaleBinding);
        }
        if self.read_paths != lease.read_paths
            || self.read_paths != package.read_paths
            || self.owned_scope != lease.owned_scope
            || !lease.owned_scope.is_subset_of(&package.owned_scope)
        {
            return Err(AdvisoryError::PermissionWidening);
        }
        if self.required_verification != package.acceptance {
            return Err(AdvisoryError::MissingVerification);
        }
        Ok(())
    }
}

fn require_active_lease(registry: &LeaseRegistry, lease: &LeaseSpec) -> Result<(), AdvisoryError> {
    if registry.get(&lease.lease_id) == Some(lease) {
        Ok(())
    } else {
        Err(AdvisoryError::StaleBinding)
    }
}
