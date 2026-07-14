mod field_types;
mod validation;

use field_types::{
    AgentId, ConnectorId, DraftStatus, NonGoal, PackageName, PackagePurpose, PackageVersion,
    SkillId, SkillRole,
};
use serde::Deserialize;

pub(crate) use field_types::RepoPath;
use validation::validate_manifest;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct DraftPackageManifest {
    name: PackageName,
    version: PackageVersion,
    status: DraftStatus,
    purpose: PackagePurpose,
    skills: Vec<DraftSkill>,
    agents: Vec<DraftAgent>,
    schemas: Vec<RepoPath>,
    fixtures: Vec<RepoPath>,
    authorable_templates: Vec<RepoPath>,
    generated_examples: Vec<RepoPath>,
    resources: Vec<RepoPath>,
    schema_catalog: RepoPath,
    optional_connectors: Vec<ConnectorId>,
    non_goals: Vec<NonGoal>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct DraftSkill {
    name: SkillId,
    path: RepoPath,
    role: SkillRole,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct DraftAgent {
    name: AgentId,
    path: RepoPath,
}

impl DraftPackageManifest {
    pub(crate) fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > super::anchored::MAX_MANIFEST_BYTES as usize {
            return Err("package manifest exceeds its byte limit".to_owned());
        }
        let mut deserializer = serde_json::Deserializer::from_slice(bytes);
        let manifest = Self::deserialize(&mut deserializer)
            .map_err(|_| "package manifest has invalid, duplicate, or unknown fields".to_owned())?;
        deserializer
            .end()
            .map_err(|_| "package manifest has trailing JSON".to_owned())?;
        validate_manifest(&manifest)?;
        Ok(manifest)
    }

    pub(crate) fn name(&self) -> &str {
        self.name.as_str()
    }

    pub(crate) fn version(&self) -> &str {
        self.version.as_str()
    }

    pub(crate) fn skills(&self) -> &[DraftSkill] {
        &self.skills
    }

    pub(crate) fn inventory_paths(&self) -> Vec<String> {
        self.skills
            .iter()
            .map(|row| row.path.as_str())
            .chain(self.agents.iter().map(|row| row.path.as_str()))
            .chain(std::iter::once(self.schema_catalog.as_str()))
            .chain(self.schemas.iter().map(RepoPath::as_str))
            .chain(self.fixtures.iter().map(RepoPath::as_str))
            .chain(self.authorable_templates.iter().map(RepoPath::as_str))
            .chain(self.generated_examples.iter().map(RepoPath::as_str))
            .chain(self.resources.iter().map(RepoPath::as_str))
            .map(str::to_owned)
            .collect()
    }
}

impl DraftSkill {
    pub(crate) fn name(&self) -> &str {
        self.name.as_str()
    }

    pub(crate) fn path(&self) -> &str {
        self.path.as_str()
    }
}
