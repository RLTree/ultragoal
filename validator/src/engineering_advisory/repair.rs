use super::AdvisoryError;
use crate::state::Repair;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairCircuitRoute {
    Retry,
    Improvement,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticRepairObservation {
    pub candidate_id: String,
    pub hypothesis: String,
    pub mechanism: String,
    pub evidence: BTreeSet<String>,
    pub ambiguous: bool,
    pub proxy_signal: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticRepairDecision {
    Continue,
    Stop,
    Escalate,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticRepairCircuit {
    pub schema_version: String,
    pub decision: SemanticRepairDecision,
    pub route: RepairCircuitRoute,
    pub repair_id: String,
    pub rerun_command_id: String,
    pub reason: String,
    pub changed_hypothesis: bool,
    pub changed_mechanism: bool,
    pub new_evidence: BTreeSet<String>,
    pub no_claim_statement: String,
}

pub const REPAIR_CIRCUIT_NO_CLAIM: &str =
    "Semantic repair advice does not own retries, improvement records, or claims.";

pub fn decide_semantic_repair(
    previous: &SemanticRepairObservation,
    current: &SemanticRepairObservation,
    repair: &Repair,
    route: RepairCircuitRoute,
) -> Result<SemanticRepairCircuit, AdvisoryError> {
    if previous.candidate_id != current.candidate_id {
        return Err(AdvisoryError::CandidateMismatch);
    }
    if current.ambiguous {
        return Err(AdvisoryError::AmbiguousRepair);
    }
    if current.proxy_signal {
        return Err(AdvisoryError::ProxyGaming);
    }
    let changed_hypothesis = previous.hypothesis != current.hypothesis;
    let changed_mechanism = previous.mechanism != current.mechanism;
    let new_evidence = current
        .evidence
        .difference(&previous.evidence)
        .cloned()
        .collect::<BTreeSet<_>>();
    if !changed_hypothesis && !changed_mechanism && new_evidence.is_empty() {
        return Ok(SemanticRepairCircuit {
            schema_version: "RepairLoopContract-v1".to_owned(),
            decision: SemanticRepairDecision::Stop,
            route,
            repair_id: repair.repair_id.clone(),
            rerun_command_id: repair.rerun_command_id.clone(),
            reason: "hypothesis, mechanism, and evidence are unchanged".to_owned(),
            changed_hypothesis,
            changed_mechanism,
            new_evidence,
            no_claim_statement: REPAIR_CIRCUIT_NO_CLAIM.to_owned(),
        });
    }
    Ok(SemanticRepairCircuit {
        schema_version: "RepairLoopContract-v1".to_owned(),
        decision: SemanticRepairDecision::Continue,
        route,
        repair_id: repair.repair_id.clone(),
        rerun_command_id: repair.rerun_command_id.clone(),
        reason: "semantic repair changed hypothesis, mechanism, or evidence".to_owned(),
        changed_hypothesis,
        changed_mechanism,
        new_evidence,
        no_claim_statement: REPAIR_CIRCUIT_NO_CLAIM.to_owned(),
    })
}
