use super::super::{
    PermitTarget, ProductError, ProductWorkspace, RootOperation, journal_head_identity,
};
use crate::orchestration::{Binding, FileJournal, JournalHead};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const ACTION_SCHEMA: &str = "OrchestrationRootActionRequest-v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RootActionReason {
    RootInterrupted,
    EffectOutcomeAmbiguous,
    InterruptedPublication,
}

/// A read-only request for root action. It is not a permit and cannot authorize
/// a mutation by itself.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RootActionRequest {
    pub schema_version: String,
    pub action_id: String,
    pub operation: RootOperation,
    pub reason: RootActionReason,
    pub authority_binding: Binding,
    pub workspace_identity: String,
    pub journal_head_identity: String,
    pub expected_head: JournalHead,
    pub snapshot_id: String,
    pub target: PermitTarget,
}

impl RootActionRequest {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        operation: RootOperation,
        reason: RootActionReason,
        authority_binding: Binding,
        workspace_identity: String,
        expected_head: JournalHead,
        snapshot_id: String,
        target: PermitTarget,
    ) -> Result<Self, ProductError> {
        let journal_head_identity = journal_head_identity(&expected_head)?;
        let mut request = Self {
            schema_version: ACTION_SCHEMA.to_owned(),
            action_id: String::new(),
            operation,
            reason,
            authority_binding,
            workspace_identity,
            journal_head_identity,
            expected_head,
            snapshot_id,
            target,
        };
        request.validate_shape()?;
        request.action_id = request.computed_id()?;
        Ok(request)
    }

    /// Revalidates request shape and journal freshness against the anchored
    /// workspace. This does not establish full-state membership and still has
    /// no authority to issue a permit; root must call the originating view's
    /// `validate_action` immediately before any separately authorized issuance.
    pub fn validate_for(&self, workspace: &ProductWorkspace) -> Result<(), ProductError> {
        self.validate_shape()?;
        workspace.verify()?;
        if self.workspace_identity != workspace.identity()
            || self.journal_head_identity != journal_head_identity(&self.expected_head)?
            || self.action_id != self.computed_id()?
        {
            return Err(ProductError::AuthorityInvalid);
        }
        match self.operation {
            RootOperation::Resume | RootOperation::Reconcile => {
                let current = FileJournal::open(workspace.root())
                    .and_then(|journal| journal.inspect())
                    .map_err(ProductError::from)?;
                if current.head != self.expected_head {
                    return Err(ProductError::ConcurrentUpdate);
                }
            }
            RootOperation::Recover => {
                let operation_id = self
                    .target
                    .operation_id
                    .as_deref()
                    .ok_or(ProductError::AuthorityOperationMismatch)?;
                let recovered_binding = self
                    .target
                    .recovered_binding
                    .as_ref()
                    .ok_or(ProductError::AuthorityOperationMismatch)?;
                FileJournal::inspect_interrupted_append(
                    workspace.root(),
                    &self.expected_head,
                    operation_id,
                    recovered_binding,
                )
                .map_err(ProductError::from)?;
            }
        }
        workspace.verify()
    }

    fn validate_shape(&self) -> Result<(), ProductError> {
        if self.schema_version != ACTION_SCHEMA
            || self.expected_head.binding != self.authority_binding
            || self.expected_head.schema_version != "OrchestrationJournalHead-v1"
            || self.expected_head.event_count == 0
            || !is_digest(&self.authority_binding.context_id)
            || !is_digest(&self.authority_binding.candidate_id)
            || !is_digest(&self.expected_head.last_event_id)
            || !is_digest(&self.expected_head.log_sha256)
            || !is_digest(&self.workspace_identity)
            || !is_digest(&self.journal_head_identity)
            || !is_digest(&self.snapshot_id)
        {
            return Err(ProductError::AuthorityInvalid);
        }
        validate_target(&self.target)?;
        let valid_operation = match self.operation {
            RootOperation::Resume => {
                self.reason == RootActionReason::RootInterrupted
                    && self.target.operation_id.is_none()
                    && self.target.recovered_binding.is_none()
            }
            RootOperation::Reconcile => {
                self.reason == RootActionReason::EffectOutcomeAmbiguous
                    && self.target.lease_id.is_some()
                    && self.target.operation_id.is_some()
                    && self.target.recovered_binding.is_none()
            }
            RootOperation::Recover => {
                self.reason == RootActionReason::InterruptedPublication
                    && self.target.operation_id.is_some()
                    && self.target.recovered_binding.is_some()
            }
        };
        if !valid_operation {
            return Err(ProductError::AuthorityOperationMismatch);
        }
        Ok(())
    }

    fn computed_id(&self) -> Result<String, ProductError> {
        #[derive(Serialize)]
        struct Commitment<'a> {
            schema_version: &'a str,
            operation: RootOperation,
            reason: RootActionReason,
            authority_binding: &'a Binding,
            workspace_identity: &'a str,
            journal_head_identity: &'a str,
            expected_head: &'a JournalHead,
            snapshot_id: &'a str,
            target: &'a PermitTarget,
        }
        let bytes = serde_json::to_vec(&Commitment {
            schema_version: &self.schema_version,
            operation: self.operation,
            reason: self.reason,
            authority_binding: &self.authority_binding,
            workspace_identity: &self.workspace_identity,
            journal_head_identity: &self.journal_head_identity,
            expected_head: &self.expected_head,
            snapshot_id: &self.snapshot_id,
            target: &self.target,
        })
        .map_err(|_| ProductError::AuthorityInvalid)?;
        Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
    }
}

fn validate_target(target: &PermitTarget) -> Result<(), ProductError> {
    if let Some(lease_id) = target.lease_id.as_deref() {
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
            if !is_digest(value) {
                return Err(ProductError::AuthorityInvalid);
            }
        } else {
            validate_identifier(value)?;
        }
    }
    if let Some(binding) = &target.recovered_binding {
        if !is_digest(&binding.context_id) || !is_digest(&binding.candidate_id) {
            return Err(ProductError::AuthorityInvalid);
        }
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

fn is_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}
