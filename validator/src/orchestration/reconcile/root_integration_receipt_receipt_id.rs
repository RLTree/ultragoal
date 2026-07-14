impl RootIntegrationReceipt {
    pub fn receipt_id(&self) -> Result<String, OrchestrationError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self).map_err(|_| OrchestrationError::InvalidReview)?;
        Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
    }

    pub fn validate(&self) -> Result<(), OrchestrationError> {
        if self.schema_version != "RootIntegrationReceipt-v1" || self.validation_receipts.is_empty()
        {
            return Err(OrchestrationError::InvalidReview);
        }
        self.base_binding.validate()?;
        self.integrated_binding.validate()?;
        self.integrated_bootstrap.validate()?;
        validate_actor_identifier(&self.root_actor)?;
        if [
            self.accepted_leases.len(),
            self.reconciled_request_digests.len(),
            self.invalidated_lease_ids.len(),
            self.applied_changes.len(),
            self.validation_receipts.len(),
        ]
        .into_iter()
        .any(|length| length > super::model::MAX_COLLECTION)
        {
            return Err(OrchestrationError::ResourceLimit);
        }
        if self.accepted_leases.is_empty() {
            return Err(OrchestrationError::InvalidReview);
        }
        for (lease_id, accepted) in &self.accepted_leases {
            validate_identifier(lease_id)?;
            accepted.validate()?;
        }
        for digest in &self.reconciled_request_digests {
            validate_digest(digest)?;
        }
        for lease_id in &self.invalidated_lease_ids {
            validate_identifier(lease_id)?;
        }
        super::validate_root_change_map(&self.applied_changes)?;
        for (check, digest) in &self.validation_receipts {
            validate_identifier(check)?;
            validate_digest(digest)?;
        }
        Ok(())
    }
}

pub fn propose_acceptance(
    current_binding: &Binding,
    package: &WorkPackage,
    lease: &LeaseSpec,
    result: &WorkerResultV1,
    review: &ReviewRecord,
) -> Result<AcceptanceProposal, OrchestrationError> {
    if &lease.binding != current_binding || &review.binding != current_binding {
        return Err(OrchestrationError::StaleBinding);
    }
    let commitment = result_commitment(current_binding, package, lease, result)?;
    let result_commitment_id = commitment.commitment_id()?;
    review.validate()?;
    if review.result_id != commitment.result_id
        || review.result_commitment_id != result_commitment_id
        || review.worker != result.worker
    {
        return Err(OrchestrationError::InvalidReview);
    }
    if review.decision != ReviewDecision::Pass || !result.unresolved_dependencies.is_empty() {
        return Err(OrchestrationError::InvalidReview);
    }
    Ok(AcceptanceProposal {
        schema_version: "AcceptanceProposal-v1".to_owned(),
        binding: commitment.binding,
        node_id: commitment.node_id,
        lease_id: commitment.lease_id,
        result_id: commitment.result_id,
        result_commitment_id,
        review_id: review.review_id()?,
        artifact_digests: commitment.artifact_digests,
        expected_root_changes: commitment.expected_root_changes,
        requested_root_change_count: commitment.requested_root_change_count,
        requested_root_changes_digest: commitment.requested_root_changes_digest,
        root_decision_required: true,
    })
}

pub(crate) fn result_commitment(
    current_binding: &Binding,
    package: &WorkPackage,
    lease: &LeaseSpec,
    result: &WorkerResultV1,
) -> Result<ResultCommitment, OrchestrationError> {
    if &lease.binding != current_binding {
        return Err(OrchestrationError::StaleBinding);
    }
    result.validate_for(lease, package)?;
    super::result_commitment_from_parts(current_binding, &package.node_id, &lease.lease_id, result)
}
