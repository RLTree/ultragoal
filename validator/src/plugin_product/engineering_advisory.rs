//! Pure validation and selection for the explicit Agentic Engineering packs.
//!
//! This module validates caller-provided observations against one exact
//! four-pack candidate. It never discovers a package, reads a cache, persists
//! selection, emits a receipt, or raises a claim. Harness remains the sole
//! operational front door.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

mod catalog;
use catalog::{BASE_SKILLS, LIFECYCLE_SKILLS, RUST_SKILLS, SYSTEMS_SKILLS};
use catalog::{ORDER, expected_skills};

pub const AGENTIC_PACK_SET_SCHEMA: &str = "AgenticPackSet-v1";
pub const AGENTIC_GATEWAY: &str = "external:harness-ultragoal";
pub const AGENTIC_VERSION: &str = "4.0.0";
pub const AGENTIC_PACK_SOURCE_COMMIT: &str = "3ebedbbf0967386057724ee166043ce5c39d6acf";
pub const AGENTIC_PACK_SOURCE_TREE: &str = "c043d15c1f83c61b65418ce8be6685f5883edac4";
pub const AGENTIC_PACK_SET_DIGEST: &str =
    "sha256:f1a4d45fe88e9c9b572609ff79630c9750fa04a552d8904db3858353b482caf7";
pub const AGENTIC_BASE_MANIFEST_DIGEST: &str =
    "sha256:c4df247b532af2cade426a751dc0c0209da4fa9f6bc9617d78b032a14b242b47";
pub const AGENTIC_LIFECYCLE_MANIFEST_DIGEST: &str =
    "sha256:89044748db8ea093c93e54ffe1d27c4d7f57f350afc2cde74ee28f1c1e6d70b5";
pub const AGENTIC_RUST_MANIFEST_DIGEST: &str =
    "sha256:0d1ec2a72e3a98d522c08493cbc2f188444afea004966484087b356015ce11ad";
pub const AGENTIC_SYSTEMS_MANIFEST_DIGEST: &str =
    "sha256:8fc500ea1deb0c8ab3231bb25b4082afc9ed0333f33708f8410dacc1f0ea9655";
// Independent package-source inventory identity: SHA-256 over sorted
// `relative-path<TAB>byte-length<TAB>content-sha256<LF>` rows. The values match
// both the selected cache and AGENTIC_PACK_SOURCE_COMMIT.
pub const AGENTIC_BASE_PACKAGE_DIGEST: &str =
    "sha256:8734b4a99bf5efc2b4db085de8e178f0aab73f6a134af9e39828a4c891cd30e8";
pub const AGENTIC_LIFECYCLE_PACKAGE_DIGEST: &str =
    "sha256:f5a4299b0f795d06357e5bc6ea70119ed11e4b856f87d95a330be9e68e8ff06c";
pub const AGENTIC_RUST_PACKAGE_DIGEST: &str =
    "sha256:c915ed6597ab65a417de0b8c19df45629f67304da6cef2cd71320ab28ea6835b";
pub const AGENTIC_SYSTEMS_PACKAGE_DIGEST: &str =
    "sha256:4b0f00ce4f40d9614afe329cd23bada85864e64be3dd6aa81cfaae252f59d0b4";

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
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgenticCandidateBinding {
    pub source_commit: String,
    pub source_tree: String,
    pub aggregate_digest: String,
    pub manifest_digests: BTreeMap<String, String>,
    pub package_digests: BTreeMap<String, String>,
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
    CompanionPackRequired,
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
        if self.packs.len() != ORDER.len() {
            return Err(AdvisoryError::CompanionPackRequired);
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
        if self.aggregate_digest != computed_digest(&self.packs)? {
            return Err(AdvisoryError::AggregateDigestMismatch);
        }
        let exact = exact_agentic_pack_set();
        if self.aggregate_digest != AGENTIC_PACK_SET_DIGEST || self.packs != exact.packs {
            return Err(AdvisoryError::CandidateDigestMismatch);
        }
        if candidate.source_commit != AGENTIC_PACK_SOURCE_COMMIT
            || candidate.source_tree != AGENTIC_PACK_SOURCE_TREE
            || candidate.aggregate_digest != self.aggregate_digest
        {
            return Err(AdvisoryError::CandidateDigestMismatch);
        }
        if candidate.manifest_digests.len() != self.packs.len() {
            return Err(AdvisoryError::CandidateDigestMismatch);
        }
        if candidate.package_digests != exact_agentic_package_digests() {
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
        if let Err(error) = self.validate_against(candidate) {
            return Err(if matches!(error, AdvisoryError::CompanionPackRequired) {
                AdvisoryError::AdviceUnavailable
            } else {
                error
            });
        }
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

/// Returns the sole Agentic package identity accepted by Harness.
///
/// These are the exact cached bytes traced to Agentic commit
/// [`AGENTIC_PACK_SOURCE_COMMIT`]; callers cannot substitute a
/// mutable same-version source tree.
pub fn exact_agentic_pack_set() -> AgenticPackSetV1 {
    let packs = [
        (ORDER[0], &BASE_SKILLS[..], AGENTIC_BASE_MANIFEST_DIGEST),
        (
            ORDER[1],
            &LIFECYCLE_SKILLS[..],
            AGENTIC_LIFECYCLE_MANIFEST_DIGEST,
        ),
        (ORDER[2], &RUST_SKILLS[..], AGENTIC_RUST_MANIFEST_DIGEST),
        (
            ORDER[3],
            &SYSTEMS_SKILLS[..],
            AGENTIC_SYSTEMS_MANIFEST_DIGEST,
        ),
    ]
    .into_iter()
    .map(|(name, skills, manifest_digest)| AgenticPackV1 {
        name: name.to_owned(),
        version: AGENTIC_VERSION.to_owned(),
        manifest_digest: manifest_digest.to_owned(),
        enabled_skills: skills.iter().map(|skill| (*skill).to_owned()).collect(),
    })
    .collect();
    AgenticPackSetV1 {
        schema_version: AGENTIC_PACK_SET_SCHEMA.to_owned(),
        gateway: AGENTIC_GATEWAY.to_owned(),
        aggregate_digest: AGENTIC_PACK_SET_DIGEST.to_owned(),
        packs,
    }
}

pub fn exact_agentic_candidate_binding() -> AgenticCandidateBinding {
    let pack_set = exact_agentic_pack_set();
    AgenticCandidateBinding {
        source_commit: AGENTIC_PACK_SOURCE_COMMIT.to_owned(),
        source_tree: AGENTIC_PACK_SOURCE_TREE.to_owned(),
        aggregate_digest: pack_set.aggregate_digest,
        manifest_digests: pack_set
            .packs
            .into_iter()
            .map(|pack| (pack.name, pack.manifest_digest))
            .collect(),
        package_digests: exact_agentic_package_digests(),
    }
}

pub fn exact_agentic_package_digests() -> BTreeMap<String, String> {
    [
        (ORDER[0], AGENTIC_BASE_PACKAGE_DIGEST),
        (ORDER[1], AGENTIC_LIFECYCLE_PACKAGE_DIGEST),
        (ORDER[2], AGENTIC_RUST_PACKAGE_DIGEST),
        (ORDER[3], AGENTIC_SYSTEMS_PACKAGE_DIGEST),
    ]
    .into_iter()
    .map(|(name, digest)| (name.to_owned(), digest.to_owned()))
    .collect()
}

pub fn owning_pack(skill: &str) -> Option<&'static str> {
    ORDER
        .iter()
        .copied()
        .find(|pack| expected_skills(pack).is_some_and(|skills| skills.contains(skill)))
}

pub fn qualify_skill(skill: &str) -> Option<String> {
    owning_pack(skill).map(|pack| format!("{pack}:{skill}"))
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
