#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewRecord {
    pub reviewer: String,
    pub worker: String,
    pub binding: Binding,
    pub result_id: String,
    pub result_commitment_id: String,
    pub decision: ReviewDecision,
    pub reproduced_commands: BTreeSet<String>,
    pub finding_codes: BTreeSet<String>,
}

impl ReviewRecord {
    pub fn review_id(&self) -> Result<String, OrchestrationError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self).map_err(|_| OrchestrationError::InvalidReview)?;
        Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
    }

    pub fn validate(&self) -> Result<(), OrchestrationError> {
        validate_actor_identifier(&self.reviewer)?;
        validate_actor_identifier(&self.worker)?;
        self.binding.validate()?;
        validate_digest(&self.result_id)?;
        validate_digest(&self.result_commitment_id)?;
        if self.reviewer == self.worker {
            return Err(OrchestrationError::ReviewerNotIndependent);
        }
        if self.reproduced_commands.is_empty() {
            return Err(OrchestrationError::InvalidReview);
        }
        if self.reproduced_commands.iter().any(|command| {
            command.is_empty() || command.len() > 4096 || command.chars().any(char::is_control)
        }) {
            return Err(OrchestrationError::InvalidReview);
        }
        for value in &self.finding_codes {
            validate_identifier(value)?;
        }
        if (self.decision == ReviewDecision::Pass && !self.finding_codes.is_empty())
            || (self.decision != ReviewDecision::Pass && self.finding_codes.is_empty())
        {
            return Err(OrchestrationError::InvalidReview);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceProposal {
    pub schema_version: String,
    pub binding: Binding,
    pub node_id: String,
    pub lease_id: String,
    pub result_id: String,
    pub result_commitment_id: String,
    pub review_id: String,
    pub artifact_digests: BTreeMap<String, String>,
    pub expected_root_changes: BTreeMap<String, String>,
    pub requested_root_change_count: usize,
    pub requested_root_changes_digest: String,
    pub root_decision_required: bool,
}

impl AcceptanceProposal {
    pub fn result_commitment_id_for(
        current_binding: &Binding,
        package: &WorkPackage,
        lease: &LeaseSpec,
        result: &WorkerResultV1,
    ) -> Result<String, OrchestrationError> {
        result_commitment(current_binding, package, lease, result)?.commitment_id()
    }

    pub fn result_commitment_id_for_parts(
        current_binding: &Binding,
        node_id: &str,
        lease_id: &str,
        result: &WorkerResultV1,
    ) -> Result<String, OrchestrationError> {
        super::result_commitment_from_parts(current_binding, node_id, lease_id, result)?
            .commitment_id()
    }

    pub fn proposal_id(&self) -> Result<String, OrchestrationError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self).map_err(|_| OrchestrationError::InvalidReview)?;
        Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
    }

    pub fn computed_result_commitment_id(&self) -> Result<String, OrchestrationError> {
        self.commitment()?.commitment_id()
    }

    pub fn computed_root_changes_digest(&self) -> Result<String, OrchestrationError> {
        super::root_change_digest(&self.expected_root_changes)
    }

    pub fn validate(&self) -> Result<(), OrchestrationError> {
        if self.schema_version != "AcceptanceProposal-v1" || !self.root_decision_required {
            return Err(OrchestrationError::InvalidReview);
        }
        if self.computed_result_commitment_id()? != self.result_commitment_id {
            return Err(OrchestrationError::InvalidReview);
        }
        validate_digest(&self.review_id)?;
        Ok(())
    }

    pub(crate) fn commitment(&self) -> Result<ResultCommitment, OrchestrationError> {
        let commitment = ResultCommitment {
            binding: self.binding.clone(),
            node_id: self.node_id.clone(),
            lease_id: self.lease_id.clone(),
            result_id: self.result_id.clone(),
            artifact_digests: self.artifact_digests.clone(),
            expected_root_changes: self.expected_root_changes.clone(),
            requested_root_change_count: self.requested_root_change_count,
            requested_root_changes_digest: self.requested_root_changes_digest.clone(),
        };
        commitment.validate()?;
        Ok(commitment)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptedLeaseIntegration {
    pub node_id: String,
    pub proposal_id: String,
    pub requested_root_changes_digest: String,
    pub requested_root_change_count: usize,
}

impl AcceptedLeaseIntegration {
    fn validate(&self) -> Result<(), OrchestrationError> {
        validate_identifier(&self.node_id)?;
        validate_digest(&self.proposal_id)?;
        validate_digest(&self.requested_root_changes_digest)?;
        if self.requested_root_change_count > super::model::MAX_COLLECTION {
            return Err(OrchestrationError::ResourceLimit);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RootIntegrationReceipt {
    pub schema_version: String,
    pub base_binding: Binding,
    pub integrated_binding: Binding,
    pub integrated_bootstrap: super::BootstrapEvidence,
    pub root_actor: String,
    pub accepted_leases: BTreeMap<String, AcceptedLeaseIntegration>,
    pub reconciled_request_digests: BTreeSet<String>,
    pub invalidated_lease_ids: BTreeSet<String>,
    pub applied_changes: BTreeMap<String, String>,
    pub validation_receipts: BTreeMap<String, String>,
}
