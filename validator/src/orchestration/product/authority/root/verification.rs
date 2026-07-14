struct RootActionPermitVerification<'a> {
    permit: &'a RootPermit,
    expected_root: &'a Actor,
    operation: RootOperation,
    binding: &'a Binding,
    workspace_identity: &'a str,
    journal_head_identity: &'a str,
    tick: u64,
    target: &'a PermitTarget,
}

struct RootReconcilePermitVerification<'a> {
    permit: &'a RootPermit,
    expected_root: &'a Actor,
    binding: &'a Binding,
    workspace_identity: &'a str,
    journal_head_identity: &'a str,
    tick: u64,
    target: &'a PermitTarget,
    resolution: &'a EffectResolution,
}

struct RootPermitVerification<'a> {
    permit: &'a RootPermit,
    expected_root: &'a Actor,
    operation: RootOperation,
    binding: &'a Binding,
    workspace_identity: &'a str,
    journal_head_identity: &'a str,
    tick: u64,
    target: &'a PermitTarget,
    decision_binding: &'a PermitDecisionBinding,
}

impl RootAuthority {
    fn verify(&self, request: RootPermitVerification<'_>) -> Result<(), ProductError> {
        let RootPermitVerification {
            permit,
            expected_root,
            operation,
            binding,
            workspace_identity,
            journal_head_identity,
            tick,
            target,
            decision_binding,
        } = request;
        if &self.root_actor != expected_root
            || permit.schema_version != AUTHORITY_SCHEMA
            || permit.root_actor != self.root_actor.as_str()
            || permit.binding != *binding
            || permit.workspace_identity != workspace_identity
            || permit.journal_head_identity != journal_head_identity
            || permit.target != *target
            || permit.decision_binding != *decision_binding
            || permit.issued_tick > tick
        {
            return Err(ProductError::AuthorityInvalid);
        }
        if permit.operation != operation {
            return Err(ProductError::AuthorityOperationMismatch);
        }
        if tick > permit.expires_tick {
            return Err(ProductError::AuthorityExpired);
        }
        validate_target(&permit.target)?;
        validate_decision_binding(permit.operation, &permit.decision_binding)?;
        let expected = self.authenticate(permit)?;
        if !constant_time_equal(expected.as_bytes(), permit.authenticator.as_bytes()) {
            return Err(ProductError::AuthorityInvalid);
        }
        Ok(())
    }

    fn verify_observation(
        &self,
        permit: &RootPermit,
        expected_root: &Actor,
        binding: &Binding,
        workspace_identity: &str,
    ) -> Result<(), ProductError> {
        self.verify(RootPermitVerification {
            permit,
            expected_root,
            operation: permit.operation,
            binding,
            workspace_identity,
            journal_head_identity: &permit.journal_head_identity,
            tick: permit.issued_tick,
            target: &permit.target,
            decision_binding: &permit.decision_binding,
        })
    }

    fn authenticate(&self, permit: &RootPermit) -> Result<String, ProductError> {
        #[derive(Serialize)]
        struct Unsigned<'a> {
            schema_version: &'a str,
            root_actor: &'a str,
            operation: RootOperation,
            binding: &'a Binding,
            workspace_identity: &'a str,
            journal_head_identity: &'a str,
            issued_tick: u64,
            expires_tick: u64,
            nonce_digest: &'a str,
            target: &'a PermitTarget,
            decision_binding: &'a PermitDecisionBinding,
        }
        let bytes = serde_json::to_vec(&Unsigned {
            schema_version: &permit.schema_version,
            root_actor: &permit.root_actor,
            operation: permit.operation,
            binding: &permit.binding,
            workspace_identity: &permit.workspace_identity,
            journal_head_identity: &permit.journal_head_identity,
            issued_tick: permit.issued_tick,
            expires_tick: permit.expires_tick,
            nonce_digest: &permit.nonce_digest,
            target: &permit.target,
            decision_binding: &permit.decision_binding,
        })
        .map_err(|_| ProductError::AuthorityInvalid)?;
        let mut hasher = Sha256::new();
        hasher.update(AUTHORITY_DOMAIN);
        hasher.update(self.key);
        hasher.update((bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
        Ok(format!("sha256:{:x}", hasher.finalize()))
    }
}

fn validate_decision_binding(
    operation: RootOperation,
    decision_binding: &PermitDecisionBinding,
) -> Result<(), ProductError> {
    match (operation, decision_binding) {
        (
            RootOperation::Reconcile,
            PermitDecisionBinding::ReconcileEffect {
                effect_resolution_commitment_id,
            },
        ) => validate_digest(effect_resolution_commitment_id),
        (RootOperation::Resume | RootOperation::Recover, PermitDecisionBinding::ActionOnly) => {
            Ok(())
        }
        _ => Err(ProductError::AuthorityOperationMismatch),
    }
}

fn validate_target(target: &PermitTarget) -> Result<(), ProductError> {
    if let Some(lease_id) = &target.lease_id {
        validate_identifier(lease_id)?;
    }
    for value in [
        target.result_commitment_id.as_deref(),
        target.operation_id.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        if value.starts_with("sha256:") {
            validate_digest(value)?;
        } else {
            validate_identifier(value)?;
        }
    }
    if let Some(binding) = &target.recovered_binding {
        validate_digest(&binding.context_id)?;
        validate_digest(&binding.candidate_id)?;
    }
    Ok(())
}

fn validate_identifier(value: &str) -> Result<(), ProductError> {
    if value.is_empty()
        || value.len() > 160
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.:/".contains(&byte))
        || value.starts_with('/')
        || value.ends_with('/')
        || value.contains("//")
        || value.contains("..")
    {
        return Err(ProductError::AuthorityInvalid);
    }
    Ok(())
}

fn validate_digest(value: &str) -> Result<(), ProductError> {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return Err(ProductError::AuthorityInvalid);
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ProductError::AuthorityInvalid);
    }
    Ok(())
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}

#[cfg(test)]
pub(crate) fn root_authority_for_test(
    root_actor: Actor,
    secret: &[u8],
) -> Result<RootAuthority, ProductError> {
    RootAuthority::from_secret(root_actor, secret)
}
