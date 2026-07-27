use super::super::{EvaluationError, EvaluationSpec, digest, valid_identifier, valid_sha256};
use std::collections::BTreeSet;

mod seal {
    pub(super) struct Seal;
}

/// Domain-sealed admission authority. Its fields and seal are deliberately not
/// exposed outside evaluation; issuance is confined to the evaluation-owned
/// root request.
pub(super) struct EvaluationAdmissionAuthority {
    live_context_id: String,
    candidate_id: String,
    spec_sha256: String,
    task_authority_id: String,
    task_principal_id: String,
    task_session_id: String,
    provenance_authority_id: String,
    provenance_principal_id: String,
    provenance_session_id: String,
    grader_authority_id: String,
    grader_principal_id: String,
    grader_session_id: String,
    binding_sha256: String,
    _seal: seal::Seal,
}

pub(super) struct EvaluationAdmissionRequest {
    task_authority_id: String,
    task_principal_id: String,
    task_session_id: String,
    provenance_authority_id: String,
    provenance_principal_id: String,
    provenance_session_id: String,
    grader_authority_id: String,
    grader_principal_id: String,
    grader_session_id: String,
}

pub(super) struct EvaluationAdmissionBinding {
    pub(super) task_authority_id: String,
    pub(super) task_principal_id: String,
    pub(super) task_session_id: String,
    pub(super) provenance_principal_id: String,
    pub(super) provenance_session_id: String,
    pub(super) grader_authority_id: String,
    pub(super) grader_principal_id: String,
    pub(super) grader_session_id: String,
}

impl EvaluationAdmissionAuthority {
    pub(super) fn issue(
        spec: &EvaluationSpec,
        request: EvaluationAdmissionRequest,
    ) -> Result<Self, EvaluationError> {
        let mut authority = Self {
            live_context_id: spec.live_context_id().to_owned(),
            candidate_id: spec.candidate_id().to_owned(),
            spec_sha256: spec.spec_sha256().to_owned(),
            task_authority_id: request.task_authority_id,
            task_principal_id: request.task_principal_id,
            task_session_id: request.task_session_id,
            provenance_authority_id: request.provenance_authority_id,
            provenance_principal_id: request.provenance_principal_id,
            provenance_session_id: request.provenance_session_id,
            grader_authority_id: request.grader_authority_id,
            grader_principal_id: request.grader_principal_id,
            grader_session_id: request.grader_session_id,
            binding_sha256: String::new(),
            _seal: seal::Seal,
        };
        authority.binding_sha256 = authority_binding_sha256(&authority);
        Ok(authority)
    }

    pub(super) fn consume(
        self,
        spec: &EvaluationSpec,
    ) -> Result<EvaluationAdmissionBinding, EvaluationError> {
        let authority_ids = [
            self.task_authority_id.as_str(),
            self.provenance_authority_id.as_str(),
            self.grader_authority_id.as_str(),
        ];
        let principals = [
            self.task_principal_id.as_str(),
            self.provenance_principal_id.as_str(),
            self.grader_principal_id.as_str(),
        ];
        let sessions = [
            self.task_session_id.as_str(),
            self.provenance_session_id.as_str(),
            self.grader_session_id.as_str(),
        ];
        let identities = authority_ids
            .iter()
            .chain(principals.iter())
            .copied()
            .collect::<BTreeSet<_>>();
        if self.live_context_id != spec.live_context_id()
            || self.candidate_id != spec.candidate_id()
            || self.spec_sha256 != spec.spec_sha256()
            || !valid_sha256(&self.live_context_id)
            || !valid_sha256(&self.candidate_id)
            || !valid_sha256(&self.spec_sha256)
            || authority_ids.iter().any(|value| !valid_identifier(value))
            || principals.iter().any(|value| !valid_identifier(value))
            || sessions.iter().any(|value| !valid_sha256(value))
            || identities.len() != authority_ids.len() + principals.len()
            || sessions.iter().copied().collect::<BTreeSet<_>>().len() != sessions.len()
            || authority_binding_sha256(&self) != self.binding_sha256
        {
            return Err(EvaluationError::new(
                "evaluation-admission-authority-invalid",
            ));
        }
        Ok(EvaluationAdmissionBinding {
            task_authority_id: self.task_authority_id,
            task_principal_id: self.task_principal_id,
            task_session_id: self.task_session_id,
            provenance_principal_id: self.provenance_principal_id,
            provenance_session_id: self.provenance_session_id,
            grader_authority_id: self.grader_authority_id,
            grader_principal_id: self.grader_principal_id,
            grader_session_id: self.grader_session_id,
        })
    }

    #[cfg(test)]
    pub(super) fn test_issue(spec: &EvaluationSpec) -> Self {
        let mut authority = Self {
            live_context_id: spec.live_context_id().to_owned(),
            candidate_id: spec.candidate_id().to_owned(),
            spec_sha256: spec.spec_sha256().to_owned(),
            task_authority_id: "evaluation-test-authority".to_owned(),
            task_principal_id: "evaluation-test-author".to_owned(),
            task_session_id: test_digest('1'),
            provenance_authority_id: "evaluation-test-provenance-authority".to_owned(),
            provenance_principal_id: "evaluation-test-provenance".to_owned(),
            provenance_session_id: test_digest('2'),
            grader_authority_id: "evaluation-test-grader-authority".to_owned(),
            grader_principal_id: "evaluation-test-grader".to_owned(),
            grader_session_id: test_digest('3'),
            binding_sha256: String::new(),
            _seal: seal::Seal,
        };
        authority.binding_sha256 = authority_binding_sha256(&authority);
        authority
    }
}

impl EvaluationAdmissionRequest {
    pub(super) fn from_values(
        task_authority_id: String,
        task_principal_id: String,
        task_session_id: String,
        provenance_authority_id: String,
        provenance_principal_id: String,
        provenance_session_id: String,
        grader_authority_id: String,
        grader_principal_id: String,
        grader_session_id: String,
    ) -> Self {
        Self {
            task_authority_id,
            task_principal_id,
            task_session_id,
            provenance_authority_id,
            provenance_principal_id,
            provenance_session_id,
            grader_authority_id,
            grader_principal_id,
            grader_session_id,
        }
    }
}

fn authority_binding_sha256(authority: &EvaluationAdmissionAuthority) -> String {
    digest(
        [
            authority.live_context_id.as_str(),
            authority.candidate_id.as_str(),
            authority.spec_sha256.as_str(),
            authority.task_authority_id.as_str(),
            authority.task_principal_id.as_str(),
            authority.task_session_id.as_str(),
            authority.provenance_authority_id.as_str(),
            authority.provenance_principal_id.as_str(),
            authority.provenance_session_id.as_str(),
            authority.grader_authority_id.as_str(),
            authority.grader_principal_id.as_str(),
            authority.grader_session_id.as_str(),
        ]
        .join("|")
        .as_bytes(),
    )
}

#[cfg(test)]
fn test_digest(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}
