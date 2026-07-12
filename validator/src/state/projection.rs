use super::catalog::{HostGoalObservation, RuntimeMetadata};
use super::ceiling::ClaimCeiling;
use super::types::{Finding, NextAction, ProductGoalState, ProductState, Repair, StateError};
use serde::Serialize;

#[derive(Serialize)]
struct InspectProjection<'a> {
    schema_version: &'static str,
    state_id: &'a str,
    context_id: &'a str,
    authority_catalog_id: &'a str,
    dependency_action_catalog_id: &'a str,
    product_goal: ProductGoalState,
    host_goal_non_authoritative: &'a HostGoalObservation,
    runtime_metadata: &'a RuntimeMetadata,
    findings: &'a [Finding],
    claim_ceilings: &'a [ClaimCeiling],
    next_action: &'a NextAction,
}

#[derive(Serialize)]
struct DiagnoseProjection<'a> {
    schema_version: &'static str,
    state_id: &'a str,
    context_id: &'a str,
    findings: &'a [Finding],
    repairs: &'a [Repair],
    claim_ceilings: &'a [ClaimCeiling],
    next_action: &'a NextAction,
}

#[derive(Serialize)]
struct NextProjection<'a> {
    schema_version: &'static str,
    state_id: &'a str,
    context_id: &'a str,
    next_action: &'a NextAction,
    claim_ceilings: &'a [ClaimCeiling],
}

#[derive(Serialize)]
struct SummaryProjection<'a> {
    schema_version: &'static str,
    state_id: &'a str,
    context_id: &'a str,
    product_goal: ProductGoalState,
    finding_count: usize,
    claim_ceilings: &'a [ClaimCeiling],
    next_action: &'a NextAction,
}

#[derive(Serialize)]
struct FindingsProjection<'a> {
    schema_version: &'static str,
    state_id: &'a str,
    context_id: &'a str,
    findings: &'a [Finding],
}

#[derive(Serialize)]
struct ClaimsProjection<'a> {
    schema_version: &'static str,
    state_id: &'a str,
    context_id: &'a str,
    claim_ceilings: &'a [ClaimCeiling],
}

impl ProductState {
    pub fn inspect_json(&self) -> Result<Vec<u8>, StateError> {
        to_json(&InspectProjection {
            schema_version: "ProductStateInspect-v1",
            state_id: &self.state_id,
            context_id: &self.context_id,
            authority_catalog_id: &self.authority_catalog_id,
            dependency_action_catalog_id: &self.dependency_action_catalog_id,
            product_goal: self.product_goal,
            host_goal_non_authoritative: &self.host_goal,
            runtime_metadata: &self.runtime_metadata,
            findings: &self.findings,
            claim_ceilings: &self.claim_ceilings,
            next_action: &self.next_action,
        })
    }

    pub fn diagnose_json(&self) -> Result<Vec<u8>, StateError> {
        to_json(&DiagnoseProjection {
            schema_version: "ProductStateDiagnose-v1",
            state_id: &self.state_id,
            context_id: &self.context_id,
            findings: &self.findings,
            repairs: &self.repairs,
            claim_ceilings: &self.claim_ceilings,
            next_action: &self.next_action,
        })
    }

    pub fn next_json(&self) -> Result<Vec<u8>, StateError> {
        to_json(&NextProjection {
            schema_version: "ProductStateNext-v1",
            state_id: &self.state_id,
            context_id: &self.context_id,
            next_action: &self.next_action,
            claim_ceilings: &self.claim_ceilings,
        })
    }

    pub fn summary_json(&self) -> Result<Vec<u8>, StateError> {
        to_json(&SummaryProjection {
            schema_version: "ProductStateSummary-v1",
            state_id: &self.state_id,
            context_id: &self.context_id,
            product_goal: self.product_goal,
            finding_count: self.findings.len(),
            claim_ceilings: &self.claim_ceilings,
            next_action: &self.next_action,
        })
    }

    pub fn findings_json(&self) -> Result<Vec<u8>, StateError> {
        to_json(&FindingsProjection {
            schema_version: "ProductStateFindings-v1",
            state_id: &self.state_id,
            context_id: &self.context_id,
            findings: &self.findings,
        })
    }

    pub fn claims_json(&self) -> Result<Vec<u8>, StateError> {
        to_json(&ClaimsProjection {
            schema_version: "ProductStateClaims-v1",
            state_id: &self.state_id,
            context_id: &self.context_id,
            claim_ceilings: &self.claim_ceilings,
        })
    }

    pub fn diagnose_finding_json(&self, identifier: &str) -> Result<Option<Vec<u8>>, StateError> {
        let exact = self
            .findings
            .iter()
            .filter(|finding| finding.finding_id == identifier)
            .collect::<Vec<_>>();
        let selected = if exact.len() == 1 {
            exact
        } else {
            let by_code = self
                .findings
                .iter()
                .filter(|finding| finding.code == identifier)
                .collect::<Vec<_>>();
            if by_code.len() == 1 {
                by_code
            } else {
                return Ok(None);
            }
        };
        let finding = selected[0];
        to_json(&DiagnoseProjection {
            schema_version: "ProductStateDiagnose-v1",
            state_id: &self.state_id,
            context_id: &self.context_id,
            findings: std::slice::from_ref(finding),
            repairs: std::slice::from_ref(&finding.repair),
            claim_ceilings: &self.claim_ceilings,
            next_action: &self.next_action,
        })
        .map(Some)
    }
}

impl crate::cli::successor::runtime::StateView for ProductState {
    fn context_id(&self) -> &str {
        self.context_id()
    }

    fn state_id(&self) -> &str {
        self.state_id()
    }

    fn finding_count(&self) -> usize {
        self.findings.len()
    }

    fn disposition(&self) -> crate::cli::successor::runtime::StateDisposition {
        use crate::cli::successor::runtime::StateDisposition;
        match self.next_action.kind {
            super::types::NextActionKind::NoOp => StateDisposition::NoAction,
            super::types::NextActionKind::Command => StateDisposition::Action,
            super::types::NextActionKind::AuthorityRequest => StateDisposition::AuthorityRequest,
            super::types::NextActionKind::NoLegalRoute => StateDisposition::NoLegalRoute,
        }
    }

    fn project(
        &self,
        projection: crate::cli::successor::runtime::StateProjection<'_>,
    ) -> Result<Option<Vec<u8>>, String> {
        use crate::cli::successor::runtime::StateProjection;
        match projection {
            StateProjection::Summary => self.summary_json().map(Some),
            StateProjection::Findings => self.findings_json().map(Some),
            StateProjection::Claims => self.claims_json().map(Some),
            StateProjection::Diagnose(None) => self.diagnose_json().map(Some),
            StateProjection::Diagnose(Some(identifier)) => self.diagnose_finding_json(identifier),
            StateProjection::Next => self.next_json().map(Some),
        }
        .map_err(|_| "state projection failed".to_owned())
    }
}

fn to_json(value: &impl Serialize) -> Result<Vec<u8>, StateError> {
    let bytes =
        serde_json::to_vec(value).map_err(|error| StateError::Serialization(error.to_string()))?;
    super::limits::bounded_projection(bytes)
}
