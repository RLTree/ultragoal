use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Registry {
    pub schema_version: u32,
    pub source_maps: Vec<String>,
    pub dependency_registries: Vec<String>,
    pub boundary_registries: Vec<String>,
    pub command_registries: Vec<String>,
    #[serde(default)]
    pub output_registries: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRow {
    pub path: String,
    pub class: String,
    pub owner: String,
    pub purpose: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyRow {
    pub manifest: String,
    #[serde(rename = "crate")]
    pub crate_name: String,
    pub version_requirement: String,
    pub owner: String,
    pub purpose: String,
    pub upstream_docs: String,
    #[serde(default)]
    pub profiles: Vec<DependencyProfile>,
    #[serde(default)]
    pub profile_registries: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyProfile {
    pub adapter_id: String,
    pub boundary_kind: String,
    pub contract: ProfileContract,
    pub timeout: OperationalPolicy,
    pub retry: OperationalPolicy,
    pub cache_invalidation_inputs: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProfileContract {
    FunctionAdapters {
        module: String,
        functions: Vec<String>,
    },
    PureOperations {
        module: String,
        symbols: Vec<String>,
    },
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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeclarationSite {
    pub module: String,
    pub symbols: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "applicability", rename_all = "snake_case", deny_unknown_fields)]
pub enum OperationalPolicy {
    Required { policy: String },
    NotApplicable { rationale: String },
}

impl OperationalPolicy {
    pub fn detail(&self) -> &str {
        match self {
            Self::Required { policy } => policy,
            Self::NotApplicable { rationale } => rationale,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundaryRow {
    pub path: String,
    pub symbol: String,
    pub authorities: Vec<String>,
    #[serde(default)]
    pub response: String,
    #[serde(default)]
    pub error: String,
    #[serde(default)]
    pub validation: Vec<String>,
    #[serde(default)]
    pub error_field: Option<String>,
    #[serde(default)]
    pub error_variant: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandRow {
    pub id: String,
    pub owner: String,
    pub entrypoint: String,
    pub kind: String,
    pub argv: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SymbolRef {
    pub path: String,
    pub symbol: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputRow {
    pub path: String,
    pub symbol: String,
    pub owner: String,
    pub purpose: String,
    pub producer: SymbolRef,
    pub validators: Vec<SymbolRef>,
    pub tests: Vec<SymbolRef>,
}

#[derive(Debug, Serialize)]
pub struct Audit {
    pub schema: String,
    pub passed: bool,
    pub governed_files: usize,
    pub failures: Vec<String>,
    pub exclusions: Vec<String>,
    pub limitations: Vec<String>,
}
