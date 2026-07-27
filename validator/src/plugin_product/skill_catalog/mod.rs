mod agentic_profiles;
mod authority_evidence;
mod model;
mod parser;
mod projection;
mod validation;
mod yaml_syntax;

pub use agentic_profiles::{
    AGENTIC_FILE_MANIFEST_DIGEST, AGENTIC_PLUGIN, AGENTIC_PLUGIN_VERSION, AgenticAdvisoryStage,
    AgenticCoInstallProfile, EXTERNAL_HARNESS_GATEWAY, all_agentic_skills, coinstall_profile,
    project_coinstall,
};
pub use model::{
    CatalogEffect, CatalogWarning, DISCOVERY_CHARACTER_LIMIT, HARNESS_FRONT_DOOR, HARNESS_PLUGIN,
    PluginIdentity, ProfileIdentity, RenderedSkill, SkillArtifact, SkillCatalogError,
    SkillCatalogProjection, SkillCatalogRequest, SkillPackage, SkillProfile,
};
pub use projection::project;

#[cfg(test)]
mod tests;
