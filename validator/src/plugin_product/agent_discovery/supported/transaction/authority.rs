use super::SupportedTransaction;
use crate::plugin_product::agent_discovery::error::{AgentDiscoveryError, AgentDiscoveryErrorId};
use crate::plugin_product::agent_discovery::filesystem::{SecureFile, digest, parse_descriptor};
use crate::plugin_product::agent_discovery::host::{
    HostAgentAuthorityTransaction, ReadOnlyEffectEnforcement, ReadOnlyEffectRequest,
};
use crate::plugin_product::agent_discovery::model::AgentAuthorityLayer;
#[cfg(test)]
use crate::plugin_product::agent_discovery::supported::report::{
    increment_capture_count, increment_effect_probe_count, record_failure,
};
use crate::plugin_product::agent_discovery::supported::roots::LayerFiles;
use crate::plugin_product::agent_discovery::supported::{conflict, invalid_binding};
use std::ffi::OsStr;

impl HostAgentAuthorityTransaction for SupportedTransaction {
    fn provenance_sha256(&self) -> &str {
        &self.provenance_sha256
    }

    fn project_root_sha256(&self) -> &str {
        &self.project_root_sha256
    }

    fn candidate_id(&self) -> &str {
        &self.candidate_id
    }

    fn session_id(&self) -> &str {
        &self.session_id
    }

    fn session_issuance_sha256(&self) -> &str {
        &self.session_issuance_sha256
    }

    fn observation_nonce_sha256(&self) -> &str {
        &self.observation_nonce_sha256
    }

    fn start_generation(&self) -> u64 {
        self.start_generation
    }

    fn current_generation(&self) -> Result<u64, ()> {
        let result = self.revalidate_all().map(|()| self.start_generation);
        #[cfg(test)]
        return result.map_err(|error| record_failure(&self.report, error.id()));
        #[cfg(not(test))]
        result.map_err(|_| ())
    }

    fn read_catalog(
        &mut self,
        layer: AgentAuthorityLayer,
        maximum: usize,
    ) -> Result<Option<Vec<u8>>, ()> {
        let result = (|| {
            let files = self.recapture(layer)?;
            let bytes = self.catalog_bytes(layer, &files)?;
            if bytes.len() > maximum {
                return Err(AgentDiscoveryError::new(
                    AgentDiscoveryErrorId::InputTooLarge,
                ));
            }
            #[cfg(test)]
            increment_capture_count(&self.report);
            Ok(Some(bytes))
        })();
        #[cfg(test)]
        return result.map_err(|error| record_failure(&self.report, error.id()));
        #[cfg(not(test))]
        result.map_err(|_| ())
    }

    fn enforce_read_only(
        &mut self,
        request: &ReadOnlyEffectRequest,
    ) -> Result<ReadOnlyEffectEnforcement, ()> {
        let result = (|| {
            if request.observation_nonce_sha256() != self.observation_nonce_sha256 {
                return Err(invalid_binding());
            }
            self.source.revalidate()?;
            let source_bytes = self
                .source
                .descriptor_bytes(request.role_name())
                .ok_or_else(conflict)?;
            if digest(source_bytes) != request.descriptor_sha256()
                || parse_descriptor(source_bytes)?.sandbox_mode != "read-only"
            {
                return Err(AgentDiscoveryError::new(
                    AgentDiscoveryErrorId::SandboxPolicyRejected,
                ));
            }
            for layer in [
                AgentAuthorityLayer::Package,
                AgentAuthorityLayer::Installed,
                AgentAuthorityLayer::Cache,
                AgentAuthorityLayer::Discovery,
            ] {
                let files = self.recapture(layer)?;
                let file = find_agent_file(&files, request.role_name()).ok_or_else(conflict)?;
                if file.sha256 != request.descriptor_sha256()
                    || parse_descriptor(&file.bytes)?.sandbox_mode != "read-only"
                {
                    return Err(AgentDiscoveryError::new(
                        AgentDiscoveryErrorId::SandboxPolicyRejected,
                    ));
                }
            }
            self.revalidate_all()?;
            #[cfg(test)]
            increment_effect_probe_count(&self.report);
            Ok(ReadOnlyEffectEnforcement::denied(request))
        })();
        #[cfg(test)]
        return result.map_err(|error| record_failure(&self.report, error.id()));
        #[cfg(not(test))]
        result.map_err(|_| ())
    }
}

fn find_agent_file<'a>(files: &'a LayerFiles, name: &str) -> Option<&'a SecureFile> {
    let expected = format!("{name}.toml");
    files
        .agents()
        .iter()
        .find(|(file_name, _)| file_name == OsStr::new(&expected))
        .map(|(_, file)| file)
}
