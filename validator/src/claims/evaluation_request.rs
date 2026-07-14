use super::decision::ClaimDecision;
use super::definition::{ClaimDefinition, ClaimDefinitions};
use super::evidence::{Actor, EvidenceEnvelope, Observation};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct ClaimEvaluation<'a> {
    pub(super) definitions: &'a ClaimDefinitions,
    pub(super) prior: &'a BTreeMap<String, ClaimDecision>,
    pub(super) accepted_candidates: &'a BTreeMap<String, String>,
    pub(super) accepted_artifacts: &'a BTreeSet<String>,
    pub(super) invalidated_evidence: &'a BTreeSet<String>,
    pub(super) observations: &'a BTreeMap<String, Observation>,
    pub(super) claim_id: &'a str,
    pub(super) context: &'a str,
    pub(super) candidate: &'a str,
    pub(super) now: u64,
    pub(super) reviewer: &'a Actor,
    pub(super) evidence_ids: &'a [String],
    pub(super) allow_semantic_models: bool,
}

pub(super) struct EvidenceEvaluation<'a> {
    pub(super) registry_digest: &'a str,
    pub(super) definition: &'a ClaimDefinition,
    pub(super) observation: &'a Observation,
    pub(super) accepted_candidates: &'a BTreeMap<String, String>,
    pub(super) accepted_artifacts: &'a BTreeSet<String>,
    pub(super) invalidated_evidence: &'a BTreeSet<String>,
    pub(super) context: &'a str,
    pub(super) candidate: &'a str,
    pub(super) now: u64,
    pub(super) reviewer: &'a Actor,
    pub(super) allow_semantic_models: bool,
}

pub(super) struct FalsePassEvaluation<'a> {
    pub(super) registry_digest: &'a str,
    pub(super) definition: &'a ClaimDefinition,
    pub(super) envelope: &'a EvidenceEnvelope,
    pub(super) accepted_artifacts: &'a BTreeSet<String>,
    pub(super) context: &'a str,
    pub(super) candidate: &'a str,
    pub(super) now: u64,
    pub(super) reviewer: &'a Actor,
    pub(super) allow_semantic_models: bool,
}
