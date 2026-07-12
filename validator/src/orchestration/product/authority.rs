use super::ProductError;
use crate::orchestration::{Actor, Binding};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::{Debug, Formatter};

const AUTHORITY_SCHEMA: &str = "OrchestrationRootPermit-v1";
const AUTHORITY_DOMAIN: &[u8] = b"harness-ultragoal/orchestration-root-permit/v1\0";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RootOperation {
    Resume,
    Reconcile,
    Recover,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PermitTarget {
    pub lease_id: Option<String>,
    pub result_commitment_id: Option<String>,
    pub operation_id: Option<String>,
    pub recovered_binding: Option<Binding>,
}

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RootPermit {
    schema_version: String,
    root_actor: String,
    operation: RootOperation,
    binding: Binding,
    workspace_identity: String,
    journal_head_identity: String,
    issued_tick: u64,
    expires_tick: u64,
    nonce_digest: String,
    target: PermitTarget,
    authenticator: String,
}

impl Debug for RootPermit {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RootPermit")
            .field("schema_version", &self.schema_version)
            .field("root_actor", &self.root_actor)
            .field("operation", &self.operation)
            .field("binding", &self.binding)
            .field("workspace_identity", &self.workspace_identity)
            .field("journal_head_identity", &self.journal_head_identity)
            .field("issued_tick", &self.issued_tick)
            .field("expires_tick", &self.expires_tick)
            .field("nonce_digest", &self.nonce_digest)
            .field("target", &self.target)
            .field("authenticator", &"[redacted]")
            .finish()
    }
}

#[derive(Clone)]
pub struct RootAuthority {
    root_actor: Actor,
    key: [u8; 32],
}

impl Debug for RootAuthority {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RootAuthority")
            .field("root_actor", &self.root_actor.as_str())
            .field("key", &"[redacted]")
            .finish()
    }
}

impl RootAuthority {
    pub(crate) fn from_secret(root_actor: Actor, secret: &[u8]) -> Result<Self, ProductError> {
        if secret.len() < 32 {
            return Err(ProductError::AuthorityInvalid);
        }
        let key: [u8; 32] = Sha256::digest(secret).into();
        Ok(Self { root_actor, key })
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn issue(
        &self,
        operation: RootOperation,
        binding: Binding,
        workspace_identity: &str,
        journal_head_identity: &str,
        issued_tick: u64,
        expires_tick: u64,
        nonce: &[u8],
        target: PermitTarget,
    ) -> Result<RootPermit, ProductError> {
        if expires_tick < issued_tick || nonce.len() < 16 {
            return Err(ProductError::AuthorityInvalid);
        }
        validate_digest(workspace_identity)?;
        validate_digest(journal_head_identity)?;
        validate_target(&target)?;
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
            authenticator: String::new(),
        };
        permit.authenticator = self.authenticate(&permit)?;
        Ok(permit)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn verify(
        &self,
        permit: &RootPermit,
        expected_root: &Actor,
        operation: RootOperation,
        binding: &Binding,
        workspace_identity: &str,
        journal_head_identity: &str,
        tick: u64,
        target: &PermitTarget,
    ) -> Result<(), ProductError> {
        if &self.root_actor != expected_root
            || permit.schema_version != AUTHORITY_SCHEMA
            || permit.root_actor != self.root_actor.as_str()
            || permit.binding != *binding
            || permit.workspace_identity != workspace_identity
            || permit.journal_head_identity != journal_head_identity
            || permit.target != *target
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
        let expected = self.authenticate(permit)?;
        if !constant_time_equal(expected.as_bytes(), permit.authenticator.as_bytes()) {
            return Err(ProductError::AuthorityInvalid);
        }
        Ok(())
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
pub fn root_authority_for_test(
    root_actor: Actor,
    secret: &[u8],
) -> Result<RootAuthority, ProductError> {
    RootAuthority::from_secret(root_actor, secret)
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub fn issue_permit_for_test(
    authority: &RootAuthority,
    operation: RootOperation,
    binding: Binding,
    workspace_identity: &str,
    journal_head_identity: &str,
    issued_tick: u64,
    expires_tick: u64,
    nonce: &[u8],
    target: PermitTarget,
) -> Result<RootPermit, ProductError> {
    authority.issue(
        operation,
        binding,
        workspace_identity,
        journal_head_identity,
        issued_tick,
        expires_tick,
        nonce,
        target,
    )
}
