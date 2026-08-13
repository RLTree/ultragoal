use super::{SkillCatalogError, SkillCatalogProjection, SkillCatalogRequest, project};
use crate::plugin_product::engineering_advisory::{
    AgenticCandidateBinding, AgenticPackSetV1, exact_agentic_candidate_binding,
    exact_agentic_pack_set, qualify_skill,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const AGENTIC_PLUGIN: &str = "agentic-engineering";
pub const EXTERNAL_HARNESS_GATEWAY: &str = "external:harness-ultragoal";
pub const AGENTIC_COINSTALL_PROFILE_SCHEMA: &str = "AgenticCoInstallProfile-v2";

const CORE: &[&str] = &[
    "agentic-engineering",
    "codex-task-contract",
    "context-repository-engineering",
    "harness-engineering",
    "loop-engineering",
    "graph-workflow-engineering",
    "multi-agent-engineering",
    "agent-evals-observability",
    "agent-security-governance",
    "agent-first-software-engineering",
    "engineering-learning-loop",
    "verification-strategy-engineering",
];
const LIFECYCLE: &[&str] = &[
    "agentic-engineering",
    "agentic-product-lifecycle",
    "authentic-use-engineering",
    "product-fitness-engineering",
    "agent-product-discovery",
    "concept-feasibility-validation",
    "requirements-systems-engineering",
    "architecture-delivery-planning",
    "software-construction-quality",
    "secure-delivery-release",
    "production-readiness-sre",
    "continuous-product-experimentation",
    "maintenance-retirement-engineering",
    "engineering-learning-loop",
    "verification-strategy-engineering",
];
const RUST: &[&str] = &[
    "agentic-engineering",
    "rust-agentic-architecture",
    "rust-agent-runtime",
    "rust-agent-durability",
    "rust-agent-protocols",
    "rust-agent-verification",
    "rust-agent-observability",
    "harness-engineering",
    "verification-strategy-engineering",
];
const ULTRAGOAL: &[&str] = &[
    "codex-task-contract",
    "agent-evals-observability",
    "agent-security-governance",
    "agentic-product-lifecycle",
    "authentic-use-engineering",
    "engineering-learning-loop",
    "verification-strategy-engineering",
    "product-fitness-engineering",
];
const ALL_AGENTIC_SKILLS: &[&str] = &[
    "agent-evals-observability",
    "agent-first-software-engineering",
    "agent-product-discovery",
    "agent-security-governance",
    "agentic-engineering",
    "agentic-product-lifecycle",
    "architecture-delivery-planning",
    "authentic-use-engineering",
    "codex-task-contract",
    "concept-feasibility-validation",
    "context-repository-engineering",
    "continuous-product-experimentation",
    "engineering-learning-loop",
    "graph-workflow-engineering",
    "harness-engineering",
    "loop-engineering",
    "maintenance-retirement-engineering",
    "multi-agent-engineering",
    "product-fitness-engineering",
    "production-readiness-sre",
    "requirements-systems-engineering",
    "rust-agent-durability",
    "rust-agent-observability",
    "rust-agent-protocols",
    "rust-agent-runtime",
    "rust-agent-verification",
    "rust-agentic-architecture",
    "secure-delivery-release",
    "software-construction-quality",
    "verification-strategy-engineering",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgenticAdvisoryStage {
    UltraGoal,
    Core,
    Lifecycle,
    Rust,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgenticCoInstallProfile {
    pub schema_version: String,
    pub stage: AgenticAdvisoryStage,
    pub source_implicit_front_door: String,
    pub external_front_door: String,
    pub source_profile_path: String,
    pub source_profile_digest: String,
    pub profile_name: String,
    pub candidate_id: String,
    pub config_digest: String,
    pub pack_set: AgenticPackSetV1,
    pub candidate_binding: AgenticCandidateBinding,
    pub selected_skills: Vec<String>,
}

impl AgenticAdvisoryStage {
    fn name(self) -> &'static str {
        match self {
            Self::UltraGoal => "ultragoal",
            Self::Core => "core",
            Self::Lifecycle => "lifecycle",
            Self::Rust => "rust",
        }
    }

    fn source_digest(self) -> &'static str {
        match self {
            Self::UltraGoal => {
                "sha256:a2d2a098a035654f7c40837b3fd58032c4e7d250cba302c23055adb446d13ea3"
            }
            Self::Core => "sha256:a9494b6dc0cdee51790d63a1273cfef138c71e67b2815e113b9dfea19df00940",
            Self::Lifecycle => {
                "sha256:6cf61fd7606da97ef185260cc3f7fb4cd17ec9fd5650a46b431b6fb4c7611e41"
            }
            Self::Rust => "sha256:84121f8c9b2a52d62746bf2c18b290ec9aa0437109e46c6a0b9b3685d366ac85",
        }
    }

    fn selected_skills(self) -> &'static [&'static str] {
        match self {
            Self::UltraGoal => ULTRAGOAL,
            Self::Core => CORE,
            Self::Lifecycle => LIFECYCLE,
            Self::Rust => RUST,
        }
    }
}

/// Builds the co-install projection, not a Codex configuration mutation.
///
/// The upstream stage profiles declare the external Harness entry and keep
/// Agentic skills explicit. The returned record is candidate-bound only when
/// the existing catalog validates it with the current package observation.
pub fn coinstall_profile(
    stage: AgenticAdvisoryStage,
    candidate_id: impl Into<String>,
    config_digest: impl Into<String>,
) -> AgenticCoInstallProfile {
    let name = stage.name();
    let source_profile_digest = stage.source_digest();
    AgenticCoInstallProfile {
        schema_version: AGENTIC_COINSTALL_PROFILE_SCHEMA.to_owned(),
        stage,
        source_implicit_front_door: EXTERNAL_HARNESS_GATEWAY.to_owned(),
        external_front_door: EXTERNAL_HARNESS_GATEWAY.to_owned(),
        source_profile_path: format!("profiles/{name}.json"),
        source_profile_digest: source_profile_digest.to_owned(),
        profile_name: format!("ultragoal-{name}"),
        candidate_id: candidate_id.into(),
        config_digest: config_digest.into(),
        pack_set: exact_agentic_pack_set(),
        candidate_binding: exact_agentic_candidate_binding(),
        selected_skills: stage
            .selected_skills()
            .iter()
            .map(|skill| qualify_skill(skill).expect("stage skill has one owning pack"))
            .collect(),
    }
}

pub fn all_agentic_skills() -> &'static [&'static str] {
    ALL_AGENTIC_SKILLS
}

pub fn is_agentic_stage_profile_digest(value: &str) -> bool {
    agentic_stage_for_profile_digest(value).is_some()
}

pub fn agentic_stage_profile_contains_qualified_skill(
    profile_digest: &str,
    qualified_skill: &str,
) -> bool {
    agentic_stage_for_profile_digest(profile_digest).is_some_and(|stage| {
        stage
            .selected_skills()
            .iter()
            .any(|skill| qualify_skill(skill).as_deref() == Some(qualified_skill))
    })
}

fn agentic_stage_for_profile_digest(value: &str) -> Option<AgenticAdvisoryStage> {
    [
        AgenticAdvisoryStage::UltraGoal,
        AgenticAdvisoryStage::Core,
        AgenticAdvisoryStage::Lifecycle,
        AgenticAdvisoryStage::Rust,
    ]
    .into_iter()
    .find(|stage| stage.source_digest() == value)
}

/// Projects only an exact Agentic co-install record through the generic catalog.
///
/// The generic catalog remains reusable for other plugins. Agentic's automatic
/// route must use this entry point so a caller cannot substitute another
/// profile, candidate, configuration, or implicit gateway during composition.
pub fn project_coinstall(
    request: &SkillCatalogRequest,
    coinstall: &AgenticCoInstallProfile,
) -> Result<SkillCatalogProjection, SkillCatalogError> {
    if coinstall.external_front_door != EXTERNAL_HARNESS_GATEWAY {
        return Err(SkillCatalogError::StaleProfile("external_front_door"));
    }
    if coinstall.candidate_id != request.candidate_id {
        return Err(SkillCatalogError::CrossCandidate(
            "agentic_profile".to_owned(),
        ));
    }
    if coinstall.config_digest != request.config_digest {
        return Err(SkillCatalogError::StaleProfile("agentic_profile_config"));
    }
    let expected = coinstall_profile(
        coinstall.stage,
        request.candidate_id.clone(),
        request.config_digest.clone(),
    );
    if coinstall != &expected {
        return Err(SkillCatalogError::StaleProfile("agentic_source_profile"));
    }
    if request.profile.is_some() {
        return Err(SkillCatalogError::StaleProfile(
            "legacy_single_plugin_profile",
        ));
    }
    coinstall
        .pack_set
        .validate_against(&coinstall.candidate_binding)
        .map_err(|_| SkillCatalogError::StaleProfile("agentic_pack_set"))?;

    let expected_packs = coinstall
        .pack_set
        .packs
        .iter()
        .map(|pack| (pack.name.as_str(), pack))
        .collect::<BTreeMap<_, _>>();
    let observed_packs = request
        .packages
        .iter()
        .filter(|package| package.selected && expected_packs.contains_key(package.plugin.as_str()))
        .map(|package| (package.plugin.as_str(), package))
        .collect::<BTreeMap<_, _>>();
    if request.packages.iter().any(|package| {
        package.selected
            && package.plugin.starts_with("agentic-engineering")
            && !expected_packs.contains_key(package.plugin.as_str())
    }) {
        return Err(SkillCatalogError::StaleProfile("unexpected_agentic_pack"));
    }
    if observed_packs.len() != expected_packs.len() {
        return Err(SkillCatalogError::ProfilePluginMissing(
            expected_packs
                .keys()
                .find(|name| !observed_packs.contains_key(**name))
                .copied()
                .unwrap_or("agentic_pack")
                .to_owned(),
        ));
    }
    let selected = coinstall
        .selected_skills
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    for (name, expected) in expected_packs {
        let observed = observed_packs[name];
        if observed.version != expected.version
            || observed.plugin_digest != expected.manifest_digest
            || coinstall.candidate_binding.package_digests.get(name)
                != Some(&observed.package_digest)
        {
            return Err(SkillCatalogError::StaleProfile("agentic_pack_identity"));
        }
        let observed_skills = observed
            .skills
            .iter()
            .map(|skill| skill.name.as_str())
            .collect::<BTreeSet<_>>();
        let expected_skills = expected
            .enabled_skills
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        if observed.skills.len() != observed_skills.len() || observed_skills != expected_skills {
            return Err(SkillCatalogError::StaleProfile("agentic_pack_skills"));
        }
        if observed.skills.iter().any(|skill| {
            let qualified = format!("{}:{}", observed.plugin, skill.name);
            skill.enabled != selected.contains(&qualified)
        }) {
            return Err(SkillCatalogError::StaleProfile("agentic_stage_selection"));
        }
    }
    let projection = project(request)?;
    let rendered = projection
        .skills
        .iter()
        .filter(|skill| observed_packs.contains_key(skill.plugin.as_str()))
        .map(|skill| format!("{}:{}", skill.plugin, skill.name))
        .collect::<BTreeSet<_>>();
    if rendered != selected {
        return Err(SkillCatalogError::StaleProfile(
            "agentic_profile_projection",
        ));
    }
    Ok(projection)
}
