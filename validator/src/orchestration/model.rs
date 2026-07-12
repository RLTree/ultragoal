use super::{CanonicalPath, OrchestrationError, OwnedScope};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const MAX_COLLECTION: usize = 512;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Actor(String);

impl Actor {
    pub fn parse(value: &str) -> Result<Self, OrchestrationError> {
        validate_actor_identifier(value)?;
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub(crate) fn validate_actor_identifier(value: &str) -> Result<(), OrchestrationError> {
    let route = value.strip_prefix('/').unwrap_or(value);
    if route.is_empty()
        || value.len() > 160
        || !route
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.:/".contains(&byte))
        || route.starts_with('/')
        || route.ends_with('/')
        || route.contains("//")
        || route.contains("..")
    {
        return Err(OrchestrationError::InvalidIdentifier);
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub context_id: String,
    pub candidate_id: String,
}

impl Binding {
    pub fn new(context_id: &str, candidate_id: &str) -> Result<Self, OrchestrationError> {
        validate_digest(context_id)?;
        validate_digest(candidate_id)?;
        Ok(Self {
            context_id: context_id.to_owned(),
            candidate_id: candidate_id.to_owned(),
        })
    }

    pub(crate) fn validate(&self) -> Result<(), OrchestrationError> {
        validate_digest(&self.context_id)?;
        validate_digest(&self.candidate_id)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BootstrapEvidence {
    pub observed_tick: u64,
    pub completed_nodes: BTreeMap<String, String>,
    pub available_tools: BTreeMap<String, String>,
    pub satisfied_prerequisites: BTreeMap<String, String>,
}

impl BootstrapEvidence {
    pub(crate) fn validate(&self) -> Result<(), OrchestrationError> {
        for values in [
            &self.completed_nodes,
            &self.available_tools,
            &self.satisfied_prerequisites,
        ] {
            if values.len() > MAX_COLLECTION {
                return Err(OrchestrationError::ResourceLimit);
            }
            for (key, digest) in values {
                validate_identifier(key)?;
                validate_digest(digest)?;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Principal {
    Worker,
    Root,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyClass {
    ReadOnly,
    IsolatedWorkspaceWrite,
    ExternalBounded,
    RootSerialized,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectClass {
    WorkspaceWrite,
    FixtureWrite,
    Process,
    Network,
    ExternalWrite,
    Destructive,
    RootAuthority,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectGrant {
    pub class: EffectClass,
    pub target: String,
}

impl EffectGrant {
    pub fn new(class: EffectClass, target: &str) -> Result<Self, OrchestrationError> {
        validate_identifier(target)?;
        Ok(Self {
            class,
            target: target.to_owned(),
        })
    }

    pub(crate) fn validate(&self) -> Result<(), OrchestrationError> {
        validate_identifier(&self.target)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkPackage {
    pub node_id: String,
    pub dependencies: BTreeSet<String>,
    pub required_tools: BTreeSet<String>,
    pub safety_class: SafetyClass,
    pub read_paths: BTreeSet<CanonicalPath>,
    pub owned_scope: OwnedScope,
    pub prerequisites: BTreeSet<String>,
    pub outputs: BTreeSet<String>,
    pub acceptance: BTreeSet<String>,
    pub claim_effect: String,
}

impl WorkPackage {
    pub fn validate(&self) -> Result<(), OrchestrationError> {
        validate_identifier(&self.node_id)?;
        bounded(&self.dependencies)?;
        bounded(&self.required_tools)?;
        bounded(&self.read_paths)?;
        bounded(&self.prerequisites)?;
        bounded(&self.outputs)?;
        bounded(&self.acceptance)?;
        for value in self
            .dependencies
            .iter()
            .chain(self.required_tools.iter())
            .chain(self.prerequisites.iter())
            .chain(self.outputs.iter())
            .chain(self.acceptance.iter())
        {
            validate_identifier(value)?;
        }
        validate_identifier(&self.claim_effect)?;
        self.owned_scope.validate()?;
        if self.owned_scope.overlaps_read_paths(&self.read_paths) {
            return Err(OrchestrationError::InvalidTransition);
        }
        if self.dependencies.contains(&self.node_id)
            || self.outputs.is_empty()
            || self.acceptance.is_empty()
        {
            return Err(OrchestrationError::InvalidTransition);
        }
        if self.safety_class == SafetyClass::ReadOnly && !self.owned_scope.is_empty() {
            return Err(OrchestrationError::InvalidTransition);
        }
        Ok(())
    }
}

pub(crate) fn validate_identifier(value: &str) -> Result<(), OrchestrationError> {
    if value.is_empty()
        || value.len() > 160
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.:/".contains(&byte))
        || value.starts_with('/')
        || value.ends_with('/')
        || value.contains("//")
        || value.contains("..")
    {
        return Err(OrchestrationError::InvalidIdentifier);
    }
    Ok(())
}

pub(crate) fn validate_digest(value: &str) -> Result<(), OrchestrationError> {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return Err(OrchestrationError::InvalidDigest);
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(OrchestrationError::InvalidDigest);
    }
    Ok(())
}

pub(crate) fn bounded<T>(values: &BTreeSet<T>) -> Result<(), OrchestrationError> {
    if values.len() > MAX_COLLECTION {
        return Err(OrchestrationError::ResourceLimit);
    }
    Ok(())
}
