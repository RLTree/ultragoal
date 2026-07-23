use super::{
    SkillCatalogError, SkillCatalogProjection, SkillCatalogRequest, SkillProfile, project,
};
use serde::{Deserialize, Serialize};

pub const AGENTIC_PLUGIN: &str = "agentic-engineering";
pub const AGENTIC_PLUGIN_VERSION: &str = "3.0.1";
pub const AGENTIC_FILE_MANIFEST_DIGEST: &str =
    "sha256:b0cf70a7db8fe86964acac725ac1a97502edf9369a8b3e8ce23b676ac78260fe";
pub const EXTERNAL_HARNESS_GATEWAY: &str = "external:harness-ultragoal";

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
    pub stage: AgenticAdvisoryStage,
    pub source_implicit_front_door: String,
    pub external_front_door: String,
    pub source_profile_path: String,
    pub source_profile_digest: String,
    pub profile: SkillProfile,
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
        stage,
        source_implicit_front_door: EXTERNAL_HARNESS_GATEWAY.to_owned(),
        external_front_door: EXTERNAL_HARNESS_GATEWAY.to_owned(),
        source_profile_path: format!("profiles/{name}.json"),
        source_profile_digest: source_profile_digest.to_owned(),
        profile: SkillProfile {
            plugin: AGENTIC_PLUGIN.to_owned(),
            name: format!("ultragoal-{name}"),
            digest: source_profile_digest.to_owned(),
            candidate_id: candidate_id.into(),
            config_digest: config_digest.into(),
            plugin_version: AGENTIC_PLUGIN_VERSION.to_owned(),
            plugin_digest: AGENTIC_FILE_MANIFEST_DIGEST.to_owned(),
            selected_skills: stage
                .selected_skills()
                .iter()
                .map(|skill| (*skill).to_owned())
                .collect(),
        },
    }
}

pub fn all_agentic_skills() -> &'static [&'static str] {
    ALL_AGENTIC_SKILLS
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
    if coinstall.profile.candidate_id != request.candidate_id {
        return Err(SkillCatalogError::CrossCandidate(
            "agentic_profile".to_owned(),
        ));
    }
    if coinstall.profile.config_digest != request.config_digest {
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
    if request.profile.as_ref() != Some(&coinstall.profile) {
        return Err(SkillCatalogError::StaleProfile(
            "agentic_profile_projection",
        ));
    }
    project(request)
}
