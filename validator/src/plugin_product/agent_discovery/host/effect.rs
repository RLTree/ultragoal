use crate::plugin_product::agent_discovery::error::{AgentDiscoveryError, AgentDiscoveryErrorId};
use crate::plugin_product::agent_discovery::protocol_codec::{
    AgentProtocolCodecRequest, encode_digest,
};
use serde::Serialize;

pub struct ReadOnlyEffectRequest {
    request_sha256: String,
    binding_sha256: String,
    role_name: String,
    descriptor_sha256: String,
    observation_nonce_sha256: String,
}

impl ReadOnlyEffectRequest {
    pub fn request_sha256(&self) -> &str {
        &self.request_sha256
    }

    pub fn binding_sha256(&self) -> &str {
        &self.binding_sha256
    }

    pub fn role_name(&self) -> &str {
        &self.role_name
    }

    pub fn descriptor_sha256(&self) -> &str {
        &self.descriptor_sha256
    }

    pub fn observation_nonce_sha256(&self) -> &str {
        &self.observation_nonce_sha256
    }

    pub(crate) fn issue(
        binding_sha256: &str,
        role_name: &str,
        descriptor_sha256: &str,
        observation_nonce_sha256: &str,
    ) -> Result<Self, AgentDiscoveryError> {
        let request_sha256 = encode_digest(AgentProtocolCodecRequest::ReadOnlyEffect {
            binding_sha256,
            role_name,
            descriptor_sha256,
            observation_nonce_sha256,
        })
        .map(|response| response.sha256())
        .map_err(|_| invalid_binding())?;
        Ok(Self {
            request_sha256,
            binding_sha256: binding_sha256.to_owned(),
            role_name: role_name.to_owned(),
            descriptor_sha256: descriptor_sha256.to_owned(),
            observation_nonce_sha256: observation_nonce_sha256.to_owned(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ReadOnlyEffectEnforcement {
    request_sha256: String,
    role_name: String,
    sandbox_mode: String,
    workspace_write_denied: bool,
    host_write_denied: bool,
    root_authority_denied: bool,
}

impl ReadOnlyEffectEnforcement {
    pub(crate) fn denied(request: &ReadOnlyEffectRequest) -> Self {
        Self {
            request_sha256: request.request_sha256.clone(),
            role_name: request.role_name.clone(),
            sandbox_mode: "read-only".to_owned(),
            workspace_write_denied: true,
            host_write_denied: true,
            root_authority_denied: true,
        }
    }

    #[cfg(test)]
    pub(crate) fn forged_write_capable(request: &ReadOnlyEffectRequest) -> Self {
        Self {
            request_sha256: request.request_sha256.clone(),
            role_name: request.role_name.clone(),
            sandbox_mode: "workspace-write".to_owned(),
            workspace_write_denied: false,
            host_write_denied: false,
            root_authority_denied: false,
        }
    }

    pub(super) fn require_exact(
        &self,
        request: &ReadOnlyEffectRequest,
    ) -> Result<(), AgentDiscoveryError> {
        let expected_request_sha256 = encode_digest(AgentProtocolCodecRequest::ReadOnlyEffect {
            binding_sha256: request.binding_sha256(),
            role_name: &request.role_name,
            descriptor_sha256: &request.descriptor_sha256,
            observation_nonce_sha256: &request.observation_nonce_sha256,
        })
        .map(|response| response.sha256())
        .map_err(|_| invalid_binding())?;
        if request.request_sha256() != expected_request_sha256
            || self.request_sha256 != request.request_sha256
            || self.role_name != request.role_name
            || self.sandbox_mode != "read-only"
            || !self.workspace_write_denied
            || !self.host_write_denied
            || !self.root_authority_denied
        {
            return Err(AgentDiscoveryError::new(
                AgentDiscoveryErrorId::SandboxPolicyRejected,
            ));
        }
        Ok(())
    }
}

fn invalid_binding() -> AgentDiscoveryError {
    AgentDiscoveryError::new(AgentDiscoveryErrorId::InvalidBinding)
}
