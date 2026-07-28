//! Pure validation and selection for the explicit Agentic Engineering packs.
//!
//! This module accepts caller-provided candidate identities only. It never
//! discovers a package, reads a cache, persists selection, emits a receipt, or
//! raises a claim. Harness remains the sole operational front door.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

mod catalog;
#[cfg(test)]
use catalog::{BASE_SKILLS, LIFECYCLE_SKILLS, RUST_SKILLS, SYSTEMS_SKILLS};
use catalog::{ORDER, expected_skills};

pub const AGENTIC_PACK_SET_SCHEMA: &str = "AgenticPackSet-v1";
pub const AGENTIC_GATEWAY: &str = "external:harness-ultragoal";
pub const AGENTIC_VERSION: &str = "4.0.0";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgenticPackSetV1 {
    pub schema_version: String,
    pub gateway: String,
    pub aggregate_digest: String,
    pub packs: Vec<AgenticPackV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgenticPackV1 {
    pub name: String,
    pub version: String,
    pub manifest_digest: String,
    pub enabled_skills: Vec<String>,
}

/// The host supplies this from the exact candidate it selected. It deliberately
/// contains no discovery location so a source tree or cache cannot become
/// authority by inference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgenticCandidateBinding {
    pub aggregate_digest: String,
    pub manifest_digests: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EngineeringAdvisorySelection {
    pub qualified_skill: String,
    pub pack_set_digest: String,
    pub proposal_only: bool,
    pub claim_effect: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdvisoryError {
    InvalidSchema,
    SecondGateway,
    BasePackRequired,
    UnknownPack,
    DuplicatePack,
    InvalidVersion,
    InvalidDigest,
    AggregateDigestMismatch,
    CandidateDigestMismatch,
    DuplicateSkill,
    SkillSetMismatch,
    InvalidQualifiedSkill,
    AdviceUnavailable,
}

impl AgenticPackSetV1 {
    pub fn validate_against(
        &self,
        candidate: &AgenticCandidateBinding,
    ) -> Result<(), AdvisoryError> {
        if self.schema_version != AGENTIC_PACK_SET_SCHEMA {
            return Err(AdvisoryError::InvalidSchema);
        }
        if self.gateway != AGENTIC_GATEWAY {
            return Err(AdvisoryError::SecondGateway);
        }
        if self.packs.is_empty() || self.packs.len() > ORDER.len() {
            return Err(AdvisoryError::BasePackRequired);
        }
        let mut names = BTreeSet::new();
        let mut skills = BTreeSet::new();
        for pack in &self.packs {
            if !ORDER.contains(&pack.name.as_str()) {
                return Err(AdvisoryError::UnknownPack);
            }
            if !names.insert(pack.name.as_str()) {
                return Err(AdvisoryError::DuplicatePack);
            }
            if pack.version != AGENTIC_VERSION {
                return Err(AdvisoryError::InvalidVersion);
            }
            if !digest(&pack.manifest_digest) {
                return Err(AdvisoryError::InvalidDigest);
            }
            let expected = expected_skills(&pack.name).ok_or(AdvisoryError::UnknownPack)?;
            let provided = pack
                .enabled_skills
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>();
            if provided.len() != pack.enabled_skills.len() {
                return Err(AdvisoryError::DuplicateSkill);
            }
            if provided != expected {
                return Err(AdvisoryError::SkillSetMismatch);
            }
            if !provided.iter().all(|skill| skills.insert(*skill)) {
                return Err(AdvisoryError::DuplicateSkill);
            }
        }
        if !names.contains("agentic-engineering") {
            return Err(AdvisoryError::BasePackRequired);
        }
        if self.aggregate_digest != computed_digest(&self.packs)? {
            return Err(AdvisoryError::AggregateDigestMismatch);
        }
        if candidate.aggregate_digest != self.aggregate_digest {
            return Err(AdvisoryError::CandidateDigestMismatch);
        }
        if candidate.manifest_digests.len() != self.packs.len() {
            return Err(AdvisoryError::CandidateDigestMismatch);
        }
        for pack in &self.packs {
            if candidate.manifest_digests.get(&pack.name) != Some(&pack.manifest_digest) {
                return Err(AdvisoryError::CandidateDigestMismatch);
            }
        }
        Ok(())
    }

    pub fn select(
        &self,
        candidate: &AgenticCandidateBinding,
        qualified_skill: &str,
    ) -> Result<EngineeringAdvisorySelection, AdvisoryError> {
        self.validate_against(candidate)?;
        let (pack_name, skill) = qualified_skill
            .split_once(':')
            .ok_or(AdvisoryError::InvalidQualifiedSkill)?;
        let pack = self
            .packs
            .iter()
            .find(|pack| pack.name == pack_name)
            .ok_or_else(|| {
                if expected_skills(pack_name).is_some() {
                    AdvisoryError::AdviceUnavailable
                } else {
                    AdvisoryError::InvalidQualifiedSkill
                }
            })?;
        if !pack.enabled_skills.iter().any(|enabled| enabled == skill) {
            return Err(AdvisoryError::InvalidQualifiedSkill);
        }
        Ok(EngineeringAdvisorySelection {
            qualified_skill: qualified_skill.to_owned(),
            pack_set_digest: self.aggregate_digest.clone(),
            proposal_only: true,
            claim_effect: "none",
        })
    }
}

fn computed_digest(packs: &[AgenticPackV1]) -> Result<String, AdvisoryError> {
    let mut ordered = packs.to_vec();
    ordered.sort_by_key(|pack| {
        ORDER
            .iter()
            .position(|name| *name == pack.name)
            .unwrap_or(usize::MAX)
    });
    let mut canonical = vec![
        AGENTIC_PACK_SET_SCHEMA.to_owned(),
        AGENTIC_GATEWAY.to_owned(),
    ];
    canonical.extend(ordered.into_iter().map(|pack| {
        let mut skills = pack.enabled_skills;
        skills.sort();
        format!(
            "{}\t{}\t{}\t{}",
            pack.name,
            pack.version,
            pack.manifest_digest,
            skills.join(",")
        )
    }));
    Ok(format!("sha256:{:x}", Sha256::digest(canonical.join("\n"))))
}

fn digest(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

#[cfg(test)]
mod tests;
