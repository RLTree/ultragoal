use super::definition::ClaimDefinitions;
use super::evidence::{Actor, Observation};
use super::guard::ClaimGuard;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionStatus {
    Passed,
    Blocked,
    Rejected,
    Invalidated,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ClaimDecision {
    pub claim_id: String,
    pub live_context_id: String,
    pub candidate_id: String,
    pub reviewer_id: String,
    pub status: DecisionStatus,
    pub evidence_ids: Vec<String>,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RejectedObservation {
    pub evidence_id: String,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Projection {
    pub registry_digest: String,
    pub decisions: Vec<ClaimDecision>,
    pub rejected_observations: Vec<RejectedObservation>,
    pub observed_evidence_ids: Vec<String>,
    pub invalidated_evidence_ids: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct DecisionLedger {
    observations: BTreeMap<String, Observation>,
    rejected: BTreeMap<String, BTreeSet<String>>,
    decisions: BTreeMap<String, ClaimDecision>,
    accepted_evidence_candidates: BTreeMap<String, String>,
    accepted_artifact_digests: BTreeSet<String>,
    invalidated_evidence_ids: BTreeSet<String>,
}

impl DecisionLedger {
    pub fn submit(&mut self, observation: Observation) -> Result<(), String> {
        let evidence_id = observation.evidence_id().to_owned();
        if self.observations.contains_key(&evidence_id) {
            return Err("claims-duplicate-evidence-id".to_owned());
        }
        self.observations.insert(evidence_id, observation);
        Ok(())
    }

    pub fn decide(
        &mut self,
        definitions: &ClaimDefinitions,
        claim_id: &str,
        live_context_id: &str,
        candidate_id: &str,
        now_unix_ms: u64,
        reviewer: &Actor,
        evidence_ids: &[String],
    ) -> ClaimDecision {
        let decision = ClaimGuard::evaluate(
            definitions,
            &self.decisions,
            &self.accepted_evidence_candidates,
            &self.accepted_artifact_digests,
            &self.invalidated_evidence_ids,
            &self.observations,
            claim_id,
            live_context_id,
            candidate_id,
            now_unix_ms,
            reviewer,
            evidence_ids,
        );
        if decision.status == DecisionStatus::Passed {
            for evidence_id in &decision.evidence_ids {
                self.accepted_evidence_candidates
                    .insert(evidence_id.clone(), candidate_id.to_owned());
                if let Some(observation) = self.observations.get(evidence_id) {
                    self.accepted_artifact_digests
                        .extend(observation.envelope().artifact_digests.iter().cloned());
                    if let Some(execution) = &observation.envelope().false_pass_execution {
                        self.accepted_artifact_digests
                            .extend(execution.execution().artifact_digests().iter().cloned());
                    }
                }
            }
        } else {
            for evidence_id in &decision.evidence_ids {
                self.rejected
                    .entry(evidence_id.clone())
                    .or_default()
                    .extend(decision.reasons.iter().cloned());
            }
        }
        self.decisions.insert(claim_id.to_owned(), decision.clone());
        decision
    }

    pub fn invalidate_for_repair(
        &mut self,
        definitions: &ClaimDefinitions,
        repaired_claim_id: &str,
    ) -> Result<Vec<String>, String> {
        let closure = definitions.dependent_closure(repaired_claim_id)?;
        let mut invalidated = Vec::new();
        for claim_id in closure {
            if let Some(decision) = self.decisions.get_mut(&claim_id) {
                if decision.status == DecisionStatus::Passed {
                    self.invalidated_evidence_ids
                        .extend(decision.evidence_ids.iter().cloned());
                    decision.status = DecisionStatus::Invalidated;
                    decision.reasons =
                        vec![format!("claims-invalidated-by-repair:{repaired_claim_id}")];
                    invalidated.push(claim_id);
                }
            }
        }
        Ok(invalidated)
    }

    pub fn projection(&self, definitions: &ClaimDefinitions) -> Projection {
        Projection {
            registry_digest: definitions.registry_digest().to_owned(),
            decisions: self.decisions.values().cloned().collect(),
            rejected_observations: self
                .rejected
                .iter()
                .map(|(evidence_id, reasons)| RejectedObservation {
                    evidence_id: evidence_id.clone(),
                    reasons: reasons.iter().cloned().collect(),
                })
                .collect(),
            observed_evidence_ids: self.observations.keys().cloned().collect(),
            invalidated_evidence_ids: self.invalidated_evidence_ids.iter().cloned().collect(),
        }
    }
}
