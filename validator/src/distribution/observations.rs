use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use crate::distribution::host_capability::{
    HostCapabilityDeclaration, HostCapabilityState, JourneyBinding,
};
use crate::distribution::model::{Capability, DistributionReport, Layer, LayerVerdict};
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeVerdict {
    Executed,
    Unsupported,
    Absent,
    DefinitionOnly,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuntimeObservation {
    context_id: String,
    candidate_id: String,
    verdict: LayerVerdict,
    runtime_verdict: RuntimeVerdict,
    binding_sha256: Option<String>,
    executable_sha256: Option<String>,
    output_sha256: Option<String>,
}

impl RuntimeObservation {
    pub fn from_report(report: &DistributionReport) -> Self {
        Self {
            context_id: report.context_id().to_owned(),
            candidate_id: report.candidate_id().to_owned(),
            verdict: report.layer(Layer::Runtime).verdict(),
            runtime_verdict: RuntimeVerdict::DefinitionOnly,
            binding_sha256: None,
            executable_sha256: None,
            output_sha256: None,
        }
    }
    pub const fn verdict(&self) -> LayerVerdict {
        self.verdict
    }
    pub const fn runtime_verdict(&self) -> RuntimeVerdict {
        self.runtime_verdict
    }
    pub fn is_current_execution(&self) -> bool {
        self.runtime_verdict == RuntimeVerdict::Executed
    }
    pub fn output_sha256(&self) -> Option<&str> {
        self.output_sha256.as_deref()
    }

    pub fn unavailable(
        binding: &JourneyBinding,
        host: &HostCapabilityDeclaration,
    ) -> Result<Self, DistributionError> {
        let runtime_verdict = match host.state(Capability::Runtime) {
            HostCapabilityState::Unsupported => RuntimeVerdict::Unsupported,
            HostCapabilityState::Absent => RuntimeVerdict::Absent,
            HostCapabilityState::Supported => {
                return Err(error(DistributionErrorId::CapabilityMismatch));
            }
        };
        Ok(Self {
            context_id: binding.package().source().context_id().into(),
            candidate_id: binding.package().source().candidate_id().into(),
            verdict: LayerVerdict::Unavailable,
            runtime_verdict,
            binding_sha256: Some(binding.binding_sha256().into()),
            executable_sha256: None,
            output_sha256: None,
        })
    }

    pub(crate) fn executed(
        binding: &JourneyBinding,
        executable_sha256: String,
        output_sha256: String,
    ) -> Self {
        Self {
            context_id: binding.package().source().context_id().into(),
            candidate_id: binding.package().source().candidate_id().into(),
            verdict: LayerVerdict::Verified,
            runtime_verdict: RuntimeVerdict::Executed,
            binding_sha256: Some(binding.binding_sha256().into()),
            executable_sha256: Some(executable_sha256),
            output_sha256: Some(output_sha256),
        }
    }
}
