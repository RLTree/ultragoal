#[cfg(test)]
pub(crate) struct RootActionPermitIssuance<'a> {
    pub(crate) operation: RootOperation,
    pub(crate) binding: Binding,
    pub(crate) workspace_identity: &'a str,
    pub(crate) journal_head_identity: &'a str,
    pub(crate) issued_tick: u64,
    pub(crate) expires_tick: u64,
    pub(crate) nonce: &'a [u8],
    pub(crate) target: PermitTarget,
}

pub(super) struct RootPermitIssuance<'a> {
    pub(super) operation: RootOperation,
    pub(super) binding: Binding,
    pub(super) workspace_identity: &'a str,
    pub(super) journal_head_identity: &'a str,
    pub(super) issued_tick: u64,
    pub(super) expires_tick: u64,
    pub(super) nonce: &'a [u8],
    pub(super) target: PermitTarget,
    pub(super) decision_binding: PermitDecisionBinding,
}

impl RootAuthority {
    #[cfg(test)]
    pub(crate) fn from_secret(root_actor: Actor, secret: &[u8]) -> Result<Self, ProductError> {
        if secret.len() < 32 {
            return Err(ProductError::AuthorityInvalid);
        }
        let key: [u8; 32] = Sha256::digest(secret).into();
        Ok(Self { root_actor, key })
    }

    #[cfg(test)]
    pub(crate) fn issue_action(
        &self,
        request: RootActionPermitIssuance<'_>,
    ) -> Result<RootPermit, ProductError> {
        let RootActionPermitIssuance {
            operation,
            binding,
            workspace_identity,
            journal_head_identity,
            issued_tick,
            expires_tick,
            nonce,
            target,
        } = request;
        if operation == RootOperation::Reconcile {
            return Err(ProductError::AuthorityOperationMismatch);
        }
        self.issue(RootPermitIssuance {
            operation,
            binding,
            workspace_identity,
            journal_head_identity,
            issued_tick,
            expires_tick,
            nonce,
            target,
            decision_binding: PermitDecisionBinding::ActionOnly,
        })
    }

    pub(super) fn issue(
        &self,
        request: RootPermitIssuance<'_>,
    ) -> Result<RootPermit, ProductError> {
        let RootPermitIssuance {
            operation,
            binding,
            workspace_identity,
            journal_head_identity,
            issued_tick,
            expires_tick,
            nonce,
            target,
            decision_binding,
        } = request;
        if expires_tick < issued_tick
            || expires_tick - issued_tick > super::MAX_PERMIT_LIFETIME
            || nonce.len() < 16
        {
            return Err(ProductError::AuthorityInvalid);
        }
        validate_digest(workspace_identity)?;
        validate_digest(journal_head_identity)?;
        validate_target(&target)?;
        validate_decision_binding(operation, &decision_binding)?;
        let nonce_digest = digest(nonce);
        let mut permit = RootPermit {
            schema_version: AUTHORITY_SCHEMA.to_owned(),
            root_actor: self.root_actor.as_str().to_owned(),
            operation,
            binding,
            workspace_identity: workspace_identity.to_owned(),
            journal_head_identity: journal_head_identity.to_owned(),
            issued_tick,
            expires_tick,
            nonce_digest,
            target,
            decision_binding,
            authenticator: String::new(),
        };
        permit.authenticator = self.authenticate(&permit)?;
        Ok(permit)
    }

    pub(super) fn verify_action(
        &self,
        request: RootActionPermitVerification<'_>,
    ) -> Result<(), ProductError> {
        let RootActionPermitVerification {
            permit,
            expected_root,
            operation,
            binding,
            workspace_identity,
            journal_head_identity,
            tick,
            target,
        } = request;
        if operation == RootOperation::Reconcile {
            return Err(ProductError::AuthorityOperationMismatch);
        }
        self.verify(RootPermitVerification {
            permit,
            expected_root,
            operation,
            binding,
            workspace_identity,
            journal_head_identity,
            tick,
            target,
            decision_binding: &PermitDecisionBinding::ActionOnly,
        })
    }

    pub(super) fn verify_reconcile(
        &self,
        request: RootReconcilePermitVerification<'_>,
    ) -> Result<(), ProductError> {
        let RootReconcilePermitVerification {
            permit,
            expected_root,
            binding,
            workspace_identity,
            journal_head_identity,
            tick,
            target,
            resolution,
        } = request;
        resolution.validate_shape().map_err(ProductError::from)?;
        if target.operation_id.as_deref() != Some(resolution.operation_id.as_str()) {
            return Err(ProductError::AuthorityOperationMismatch);
        }
        let effect_resolution_commitment_id =
            resolution.commitment_id().map_err(ProductError::from)?;
        self.verify(RootPermitVerification {
            permit,
            expected_root,
            operation: RootOperation::Reconcile,
            binding,
            workspace_identity,
            journal_head_identity,
            tick,
            target,
            decision_binding: &PermitDecisionBinding::ReconcileEffect {
                effect_resolution_commitment_id,
            },
        })
    }
}
