mod agentic_profiles;
mod authority_evidence;
mod model;
mod parser;
mod projection;
mod validation;
mod yaml_syntax;

pub use agentic_profiles::{
    AGENTIC_COINSTALL_PROFILE_SCHEMA, AGENTIC_PLUGIN, AgenticAdvisoryStage,
    AgenticCoInstallProfile, EXTERNAL_HARNESS_GATEWAY,
    agentic_stage_profile_contains_qualified_skill, all_agentic_skills, coinstall_profile,
    is_agentic_stage_profile_digest, project_coinstall,
};
pub use model::{
    CatalogEffect, CatalogWarning, DISCOVERY_CHARACTER_LIMIT, HARNESS_FRONT_DOOR, HARNESS_PLUGIN,
    PluginIdentity, ProfileIdentity, RenderedSkill, SkillArtifact, SkillCatalogError,
    SkillCatalogProjection, SkillCatalogRequest, SkillPackage, SkillProfile,
};
pub use projection::project;

#[cfg(test)]
mod tests;
