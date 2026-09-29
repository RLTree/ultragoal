use serde::{Deserialize, Serialize};

pub const DISCOVERY_CHARACTER_LIMIT: usize = 8_000;
pub const HARNESS_PLUGIN: &str = "harness-ultragoal";
pub const HARNESS_FRONT_DOOR: &str = "harness-ultragoal";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillCatalogRequest {
    pub candidate_id: String,
    pub config_digest: String,
    pub profile: Option<SkillProfile>,
    pub packages: Vec<SkillPackage>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillProfile {
    pub plugin: String,
    pub name: String,
    pub digest: String,
    pub candidate_id: String,
    pub config_digest: String,
    pub plugin_version: String,
    pub plugin_digest: String,
    pub selected_skills: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillPackage {
    pub plugin: String,
    pub version: String,
    pub plugin_digest: String,
    pub package_digest: String,
    pub candidate_id: String,
    pub config_digest: String,
    pub selected: bool,
    pub skills: Vec<SkillArtifact>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillArtifact {
    pub name: String,
    pub path: String,
    pub openai_yaml: String,
    pub enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogEffect {
    Read,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogWarning {
    OmittedSkill { plugin: String, skill: String },
    LegacyAliasExcluded { plugin: String, skill: String },
    TruncatedDescription { plugin: String, skill: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenderedSkill {
    pub plugin: String,
    pub version: String,
    pub name: String,
    pub path: String,
    pub display_name: String,
    pub short_description: String,
    pub default_prompt: String,
    pub allow_implicit_invocation: bool,
    pub source_digest: String,
    pub rendered_characters: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillCatalogProjection {
    pub schema_version: String,
    pub candidate_id: String,
    pub config_digest: String,
    pub profile: Option<ProfileIdentity>,
    pub plugins: Vec<PluginIdentity>,
    pub skills: Vec<RenderedSkill>,
    pub enabled_skill_count: usize,
    pub rendered_characters: usize,
    pub character_limit: usize,
    pub headroom: usize,
    pub implicit_gateways: Vec<String>,
    pub warnings: Vec<CatalogWarning>,
    pub effect: CatalogEffect,
    pub claim_effect: bool,
    pub projection_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileIdentity {
    pub plugin: String,
    pub name: String,
    pub digest: String,
    pub candidate_id: String,
    pub config_digest: String,
    pub plugin_version: String,
    pub plugin_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginIdentity {
    pub plugin: String,
    pub version: String,
    pub plugin_digest: String,
    pub package_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SkillCatalogError {
    InvalidDigest(&'static str),
    EmptyPackages,
    DuplicatePackage(String),
    CrossCandidate(String),
    ConfigMismatch(String),
    PluginVersionMismatch(String),
    ProfilePluginMissing(String),
    StaleProfile(&'static str),
    DuplicateProfileSkill(String),
    UnknownProfileSkill(String),
    ExtraProfileSkill(String),
    OmittedProfileSkill(String),
    DuplicateSkill(String),
    InvalidPath(String),
    ParseSkill(String, String),
    LegacyAliasSelected(String),
    InvalidImplicitPolicy(String),
    UnexpectedImplicitSkill(String),
    MissingImplicitGateway,
    MultipleImplicitGateways(Vec<String>),
    OverBudget { estimate: usize, limit: usize },
}
