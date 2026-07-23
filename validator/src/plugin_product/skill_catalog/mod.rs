mod authority_evidence;
mod model;
mod parser;
mod projection;
mod validation;
mod yaml_syntax;

pub(crate) use authority_evidence::legacy_aliases;
pub use model::{
    CatalogEffect, CatalogWarning, DISCOVERY_CHARACTER_LIMIT, HARNESS_FRONT_DOOR, HARNESS_PLUGIN,
    PluginIdentity, ProfileIdentity, RenderedSkill, SkillArtifact, SkillCatalogError,
    SkillCatalogProjection, SkillCatalogRequest, SkillPackage, SkillProfile,
};
pub use projection::project;

#[cfg(test)]
mod tests;
