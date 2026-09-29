use super::repair::Repair;
use super::state_authority::AuthorityRequirement;
use crate::context::EffectClass;
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FindingSeverity {
    Blocked,
    Error,
    Warning,
    Info,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CeilingReduction {
    pub claim_id: String,
    pub dimensions: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Scope {
    pub surface: String,
    pub relative_path: Option<String>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum FindingSource {
    LiveContext {
        context_id: String,
    },
    AuthorityCatalog {
        catalog_id: String,
        code: String,
    },
    DependencyCatalog {
        catalog_id: String,
        observation_id: String,
    },
    RuntimeMetadata {
        field: String,
    },
    StatePolicy {
        catalog_id: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Finding {
    pub finding_id: String,
    pub code: String,
    pub severity: FindingSeverity,
    pub source: FindingSource,
    pub scope: Scope,
    pub dependency_ids: BTreeSet<String>,
    pub cause: String,
    pub effect: EffectClass,
    pub authority: AuthorityRequirement,
    pub repair: Repair,
    pub affected_claims: BTreeSet<String>,
    pub ceiling_reductions: Vec<CeilingReduction>,
}
