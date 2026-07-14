pub struct HostAgentAuthorityRequest {
    provenance_sha256: String,
    project_root_sha256: String,
    candidate_id: String,
    session_id: String,
    session_issuance_sha256: String,
    observation_nonce_sha256: String,
}

impl HostAgentAuthorityRequest {
    pub fn provenance_sha256(&self) -> &str {
        &self.provenance_sha256
    }

    pub fn project_root_sha256(&self) -> &str {
        &self.project_root_sha256
    }

    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn session_issuance_sha256(&self) -> &str {
        &self.session_issuance_sha256
    }

    pub fn observation_nonce_sha256(&self) -> &str {
        &self.observation_nonce_sha256
    }

    pub(crate) fn issue(
        provenance_sha256: String,
        project_root_sha256: String,
        candidate_id: String,
        session_id: String,
        session_issuance_sha256: String,
        observation_nonce_sha256: String,
    ) -> Self {
        Self {
            provenance_sha256,
            project_root_sha256,
            candidate_id,
            session_id,
            session_issuance_sha256,
            observation_nonce_sha256,
        }
    }
}
