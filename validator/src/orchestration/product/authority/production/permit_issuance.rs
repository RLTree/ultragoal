use super::super::super::command::RootActionRequest;
use crate::orchestration::EffectResolution;
use getrandom::fill;

impl ProductionRootAuthority {
    pub(crate) fn issue_validated_action(
        &self,
        action: &RootActionRequest,
        issued_tick: u64,
        expires_tick: u64,
    ) -> Result<RootPermit, ProductError> {
        if action.operation == RootOperation::Reconcile {
            return Err(ProductError::AuthorityOperationMismatch);
        }
        self.issue(
            action,
            issued_tick,
            expires_tick,
            PermitDecisionBinding::ActionOnly,
        )
    }

    pub(crate) fn issue_validated_reconcile(
        &self,
        action: &RootActionRequest,
        issued_tick: u64,
        expires_tick: u64,
        resolution: &EffectResolution,
    ) -> Result<RootPermit, ProductError> {
        resolution.validate_shape().map_err(ProductError::from)?;
        if action.operation != RootOperation::Reconcile
            || action.target.operation_id.as_deref() != Some(resolution.operation_id.as_str())
        {
            return Err(ProductError::AuthorityOperationMismatch);
        }
        self.issue(
            action,
            issued_tick,
            expires_tick,
            PermitDecisionBinding::ReconcileEffect {
                effect_resolution_commitment_id: resolution
                    .commitment_id()
                    .map_err(ProductError::from)?,
            },
        )
    }

    fn issue(
        &self,
        action: &RootActionRequest,
        issued_tick: u64,
        expires_tick: u64,
        decision_binding: PermitDecisionBinding,
    ) -> Result<RootPermit, ProductError> {
        if expires_tick < issued_tick || expires_tick - issued_tick > MAX_PERMIT_LIFETIME {
            return Err(ProductError::AuthorityInvalid);
        }
        let mut nonce = [0_u8; NONCE_BYTES];
        fill(&mut nonce).map_err(|_| ProductError::AuthorityStoreInvalid)?;
        let permit = self.authority.issue(RootPermitIssuance {
            operation: action.operation,
            binding: action.authority_binding.clone(),
            workspace_identity: &action.workspace_identity,
            journal_head_identity: &action.journal_head_identity,
            issued_tick,
            expires_tick,
            nonce: &nonce,
            target: action.target.clone(),
            decision_binding,
        })?;
        nonce.fill(0);
        self.ledger
            .issue(&permit_id(&permit)?, &causal_slot_id(&permit)?)?;
        Ok(permit)
    }
}
