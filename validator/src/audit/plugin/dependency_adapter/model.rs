use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DependencyRegistry {
    pub(crate) schema_version: String,
    pub(crate) rows: Vec<DependencyRow>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DependencyRow {
    #[serde(rename = "crate")]
    pub(crate) crate_name: String,
    pub(crate) version_requirement: String,
    pub(crate) owner: String,
    pub(crate) purpose: String,
    pub(crate) upstream_docs: String,
    pub(crate) profiles: Vec<DependencyProfile>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DependencyProfile {
    pub(crate) adapter_id: String,
    pub(crate) boundary_kind: BoundaryKind,
    pub(crate) contract: ProfileContract,
    pub(crate) timeout: OperationalPolicy,
    pub(crate) retry: OperationalPolicy,
    pub(crate) cache_invalidation_inputs: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BoundaryKind {
    TypedParser,
    TypedEmitter,
    EffectAdapter,
    CryptoAdapter,
    FilesystemAdapter,
    TypedContractDerive,
    CliContractDeclaration,
}

impl BoundaryKind {
    pub(crate) fn requires_owned_adapter(self) -> bool {
        matches!(
            self,
            Self::TypedParser
                | Self::TypedEmitter
                | Self::EffectAdapter
                | Self::CryptoAdapter
                | Self::FilesystemAdapter
        )
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum ProfileContract {
    OwnedAdapter {
        module: String,
        request: String,
        response: String,
        error: String,
    },
    TypedDeclarations {
        declarations: Vec<DeclarationSite>,
    },
}

impl ProfileContract {
    pub(crate) fn modules(&self) -> Vec<&str> {
        match self {
            Self::OwnedAdapter { module, .. } => vec![module],
            Self::TypedDeclarations { declarations } => declarations
                .iter()
                .map(|site| site.module.as_str())
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeclarationSite {
    pub(crate) module: String,
    pub(crate) symbols: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "applicability", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum OperationalPolicy {
    Required { policy: String },
    NotApplicable { rationale: String },
}

impl OperationalPolicy {
    pub(crate) fn detail(&self) -> &str {
        match self {
            Self::Required { policy } => policy,
            Self::NotApplicable { rationale } => rationale,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DirectDependency {
    pub(crate) crate_name: String,
    pub(crate) version_requirement: String,
}
