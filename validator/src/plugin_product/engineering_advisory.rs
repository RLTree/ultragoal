//! Pure validation and selection for the explicit Agentic Engineering packs.
//!
//! This module accepts caller-provided candidate identities only. It never
//! discovers a package, reads a cache, persists selection, emits a receipt, or
//! raises a claim. Harness remains the sole operational front door.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const AGENTIC_PACK_SET_SCHEMA: &str = "AgenticPackSet-v1";
pub const AGENTIC_GATEWAY: &str = "external:harness-ultragoal";
pub const AGENTIC_VERSION: &str = "4.0.0";

const ORDER: [&str; 4] = [
    "agentic-engineering",
    "agentic-engineering-lifecycle",
    "agentic-engineering-rust",
    "agentic-engineering-systems",
];

const BASE_SKILLS: [&str; 8] = [
    "agent-evals-observability",
    "agent-security-governance",
    "agentic-product-lifecycle",
    "authentic-use-engineering",
    "codex-task-contract",
    "engineering-learning-loop",
    "product-fitness-engineering",
    "verification-strategy-engineering",
];
const LIFECYCLE_SKILLS: [&str; 10] = [
    "agentic-engineering",
    "agent-product-discovery",
    "architecture-delivery-planning",
    "concept-feasibility-validation",
    "continuous-product-experimentation",
    "maintenance-retirement-engineering",
    "production-readiness-sre",
    "requirements-systems-engineering",
    "secure-delivery-release",
    "software-construction-quality",
];
const RUST_SKILLS: [&str; 7] = [
    "harness-engineering",
    "rust-agent-durability",
    "rust-agent-observability",
    "rust-agent-protocols",
    "rust-agent-runtime",
    "rust-agent-verification",
    "rust-agentic-architecture",
];
const SYSTEMS_SKILLS: [&str; 5] = [
    "agent-first-software-engineering",
    "context-repository-engineering",
    "graph-workflow-engineering",
    "loop-engineering",
    "multi-agent-engineering",
];

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

fn expected_skills(name: &str) -> Option<BTreeSet<&'static str>> {
    let skills = match name {
        "agentic-engineering" => &BASE_SKILLS[..],
        "agentic-engineering-lifecycle" => &LIFECYCLE_SKILLS[..],
        "agentic-engineering-rust" => &RUST_SKILLS[..],
        "agentic-engineering-systems" => &SYSTEMS_SKILLS[..],
        _ => return None,
    };
    Some(skills.iter().copied().collect())
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
        format!(
            "{}\t{}\t{}\t{}",
            pack.name,
            pack.version,
            pack.manifest_digest,
            pack.enabled_skills.join(",")
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
mod tests {
    use super::*;

    fn pack(name: &str, skills: &[&str], index: u8) -> AgenticPackV1 {
        AgenticPackV1 {
            name: name.to_owned(),
            version: AGENTIC_VERSION.to_owned(),
            manifest_digest: format!("sha256:{index:064x}"),
            enabled_skills: skills.iter().map(|value| (*value).to_owned()).collect(),
        }
    }

    fn set(packs: Vec<AgenticPackV1>) -> AgenticPackSetV1 {
        let aggregate_digest = computed_digest(&packs).unwrap();
        AgenticPackSetV1 {
            schema_version: AGENTIC_PACK_SET_SCHEMA.to_owned(),
            gateway: AGENTIC_GATEWAY.to_owned(),
            aggregate_digest,
            packs,
        }
    }

    fn binding(value: &AgenticPackSetV1) -> AgenticCandidateBinding {
        AgenticCandidateBinding {
            aggregate_digest: value.aggregate_digest.clone(),
            manifest_digests: value
                .packs
                .iter()
                .map(|pack| (pack.name.clone(), pack.manifest_digest.clone()))
                .collect(),
        }
    }

    #[test]
    fn base_only_is_a_valid_explicit_pack_set() {
        let value = set(vec![pack("agentic-engineering", &BASE_SKILLS, 1)]);
        assert_eq!(value.validate_against(&binding(&value)), Ok(()));
        assert_eq!(
            value
                .select(&binding(&value), "agentic-engineering:codex-task-contract")
                .unwrap(),
            EngineeringAdvisorySelection {
                qualified_skill: "agentic-engineering:codex-task-contract".to_owned(),
                pack_set_digest: value.aggregate_digest.clone(),
                proposal_only: true,
                claim_effect: "none",
            }
        );
    }

    #[test]
    fn full_pack_set_has_the_exact_thirty_skill_union() {
        let value = set(vec![
            pack("agentic-engineering", &BASE_SKILLS, 1),
            pack("agentic-engineering-lifecycle", &LIFECYCLE_SKILLS, 2),
            pack("agentic-engineering-rust", &RUST_SKILLS, 3),
            pack("agentic-engineering-systems", &SYSTEMS_SKILLS, 4),
        ]);
        assert_eq!(value.validate_against(&binding(&value)), Ok(()));
        assert_eq!(
            value
                .select(
                    &binding(&value),
                    "agentic-engineering-rust:harness-engineering"
                )
                .unwrap()
                .pack_set_digest,
            value.aggregate_digest
        );
    }

    #[test]
    fn missing_or_substituted_packs_fail_closed() {
        let value = set(vec![pack("agentic-engineering", &BASE_SKILLS, 1)]);
        let mut stale = binding(&value);
        stale.manifest_digests.insert(
            "agentic-engineering".to_owned(),
            "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_owned(),
        );
        assert_eq!(
            value.validate_against(&stale),
            Err(AdvisoryError::CandidateDigestMismatch)
        );
        assert_eq!(
            value.select(
                &binding(&value),
                "agentic-engineering-rust:harness-engineering"
            ),
            Err(AdvisoryError::AdviceUnavailable)
        );
    }

    #[test]
    fn duplicate_or_omitted_skills_and_second_gateway_are_rejected() {
        let mut duplicate = set(vec![pack("agentic-engineering", &BASE_SKILLS, 1)]);
        duplicate.packs[0]
            .enabled_skills
            .push(BASE_SKILLS[0].to_owned());
        assert_eq!(
            duplicate.validate_against(&binding(&duplicate)),
            Err(AdvisoryError::DuplicateSkill)
        );

        let mut omitted = set(vec![pack("agentic-engineering", &BASE_SKILLS, 1)]);
        omitted.packs[0].enabled_skills.pop();
        assert_eq!(
            omitted.validate_against(&binding(&omitted)),
            Err(AdvisoryError::SkillSetMismatch)
        );

        let mut second_gateway = set(vec![pack("agentic-engineering", &BASE_SKILLS, 1)]);
        second_gateway.gateway = "external:other".to_owned();
        assert_eq!(
            second_gateway.validate_against(&binding(&second_gateway)),
            Err(AdvisoryError::SecondGateway)
        );
    }
}
