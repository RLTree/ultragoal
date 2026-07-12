use super::super::ProductError;
use super::{OrchestrationFinding, OrchestrationStateView, RootActionRequest};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosisStatus {
    Clear,
    Finding,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrchestrationDiagnosis {
    pub schema_version: String,
    pub diagnosis_id: String,
    pub state_id: String,
    pub status: DiagnosisStatus,
    pub finding: Option<OrchestrationFinding>,
    pub root_action_request: Option<RootActionRequest>,
}

pub fn diagnose(
    view: &OrchestrationStateView,
    finding_id: Option<&str>,
) -> Result<Option<OrchestrationDiagnosis>, ProductError> {
    view.require_authority()?;
    let selected = match finding_id {
        Some(id) => match view
            .findings
            .iter()
            .find(|finding| finding.finding_id == id)
        {
            Some(finding) => Some(finding.clone()),
            None => return Ok(None),
        },
        None => view.findings.first().cloned(),
    };
    let action = selected.as_ref().and_then(|finding| {
        view.root_action_requests
            .iter()
            .find(|action| {
                action.target.lease_id == finding.lease_id
                    && action.target.operation_id == finding.operation_id
                    || finding.operation_id.is_none()
                        && action.reason == super::RootActionReason::RootInterrupted
                        && finding.kind == super::FindingKind::RootInterrupted
            })
            .cloned()
    });
    let status = if selected.is_some() {
        DiagnosisStatus::Finding
    } else {
        DiagnosisStatus::Clear
    };
    let bytes = serde_json::to_vec(&(&view.state_id, status, &selected, &action))
        .map_err(|_| ProductError::StaleCandidate)?;
    Ok(Some(OrchestrationDiagnosis {
        schema_version: "OrchestrationDiagnosis-v1".to_owned(),
        diagnosis_id: format!("sha256:{:x}", Sha256::digest(bytes)),
        state_id: view.state_id.clone(),
        status,
        finding: selected,
        root_action_request: action,
    }))
}
