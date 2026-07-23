use super::AdvisoryError;
use crate::orchestration::{
    Binding, CanonicalPath, LeaseSpec, Principal, SafetyClass, WorkPackage,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const TASK_EVIDENCE_NO_CLAIM: &str =
    "This advisory packet is proposal-only and cannot promote readiness, release, or completion.";

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
    pub read_paths: BTreeSet<CanonicalPath>,
    pub owned_scope: crate::orchestration::OwnedScope,
    pub required_verification: BTreeSet<String>,
    pub claim_effect: String,
    pub no_claim_statement: String,
}

impl TaskEvidencePacket {
    pub fn from_work_package(
        package: &WorkPackage,
        lease: &LeaseSpec,
    ) -> Result<Self, AdvisoryError> {
        package
            .validate()
            .map_err(|_| AdvisoryError::PermissionWidening)?;
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
            read_paths: lease.read_paths.clone(),
            owned_scope: lease.owned_scope.clone(),
            required_verification: package.acceptance.clone(),
            claim_effect: package.claim_effect.clone(),
            no_claim_statement: TASK_EVIDENCE_NO_CLAIM.to_owned(),
        };
        packet.validate_against(package, lease)?;
        Ok(packet)
    }

    pub fn validate_against(
        &self,
        package: &WorkPackage,
        lease: &LeaseSpec,
    ) -> Result<(), AdvisoryError> {
        if self.schema_version != "TaskEvidencePacket-v1"
            || self.no_claim_statement != TASK_EVIDENCE_NO_CLAIM
            || self.binding != lease.binding
            || self.node_id != package.node_id
            || self.node_id != lease.node_id
            || self.run_id != lease.run_id
            || self.lease_id != lease.lease_id
            || self.worker != lease.owner.as_str()
            || self.principal != lease.principal
            || self.safety_class != lease.safety_class
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
