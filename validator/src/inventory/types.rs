use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;

pub(crate) const MAX_CATALOG_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuthorityState {
    Canonical,
    Projection,
    Legacy,
    Context,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActiveStatus {
    Active,
    Definition,
    Required,
    Missing,
    Retired,
    ContextOnly,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FindingSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct InventoryFinding {
    pub code: String,
    pub severity: FindingSeverity,
    pub entry_id: Option<String>,
    pub relative_path: Option<String>,
    pub message: String,
}

impl InventoryFinding {
    pub(crate) fn error(code: &str, id: Option<&str>, path: Option<&str>, message: String) -> Self {
        Self {
            code: code.to_owned(),
            severity: FindingSeverity::Error,
            entry_id: id.map(ToOwned::to_owned),
            relative_path: path.map(ToOwned::to_owned),
            message,
        }
    }

    pub(crate) fn warning(
        code: &str,
        id: Option<&str>,
        path: Option<&str>,
        message: String,
    ) -> Self {
        Self {
            code: code.to_owned(),
            severity: FindingSeverity::Warning,
            entry_id: id.map(ToOwned::to_owned),
            relative_path: path.map(ToOwned::to_owned),
            message,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct InventoryEntry {
    pub stable_id: String,
    pub kind: String,
    pub owner_role: String,
    pub relative_path: String,
    pub digest_sha256: String,
    pub unix_mode: Option<u32>,
    pub authority_state: AuthorityState,
    pub active_status: ActiveStatus,
    pub generator: Option<String>,
    pub input_provenance: Vec<String>,
    pub references: Vec<String>,
}

impl InventoryEntry {
    pub(crate) fn normalize(&mut self) {
        self.input_provenance.sort();
        self.input_provenance.dedup();
        self.references.sort();
        self.references.dedup();
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GeneratedSurfaceIndex {
    entries: Vec<InventoryEntry>,
}

impl GeneratedSurfaceIndex {
    pub fn entries(&self) -> &[InventoryEntry] {
        &self.entries
    }
    pub(crate) fn new(entries: Vec<InventoryEntry>) -> Self {
        Self { entries }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AuthorityCatalog {
    schema_version: &'static str,
    catalog_id: String,
    context_id: String,
    contract_id: String,
    source_registry_counts: BTreeMap<String, usize>,
    entries: Vec<InventoryEntry>,
    findings: Vec<InventoryFinding>,
    generated_surfaces: GeneratedSurfaceIndex,
}

impl AuthorityCatalog {
    pub fn catalog_id(&self) -> &str {
        &self.catalog_id
    }
    pub fn context_id(&self) -> &str {
        &self.context_id
    }
    pub fn contract_id(&self) -> &str {
        &self.contract_id
    }
    pub fn source_registry_counts(&self) -> &BTreeMap<String, usize> {
        &self.source_registry_counts
    }
    pub fn entries(&self) -> &[InventoryEntry] {
        &self.entries
    }
    pub fn findings(&self) -> &[InventoryFinding] {
        &self.findings
    }
    pub fn has_error_findings(&self) -> bool {
        self.findings
            .iter()
            .any(|finding| finding.severity == FindingSeverity::Error)
    }
    pub fn generated_surfaces(&self) -> &GeneratedSurfaceIndex {
        &self.generated_surfaces
    }
    pub fn to_canonical_json(&self) -> Result<Vec<u8>, InventoryError> {
        let bytes = serde_json::to_vec(self)
            .map_err(|error| InventoryError::Serialization(error.to_string()))?;
        if bytes.len() > MAX_CATALOG_BYTES {
            return Err(InventoryError::Serialization(format!(
                "catalog exceeds {MAX_CATALOG_BYTES} bytes"
            )));
        }
        Ok(bytes)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        catalog_id: String,
        context_id: String,
        contract_id: String,
        source_registry_counts: BTreeMap<String, usize>,
        entries: Vec<InventoryEntry>,
        findings: Vec<InventoryFinding>,
        generated_surfaces: GeneratedSurfaceIndex,
    ) -> Self {
        Self {
            schema_version: "AuthorityCatalog-v1",
            catalog_id,
            context_id,
            contract_id,
            source_registry_counts,
            entries,
            findings,
            generated_surfaces,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProjectionComparison {
    pub matches: bool,
    pub expected_sha256: String,
    pub actual_sha256: String,
    pub findings: Vec<InventoryFinding>,
}

#[derive(Debug)]
pub enum InventoryError {
    Context(String),
    Io { path: PathBuf, message: String },
    Json { path: PathBuf, message: String },
    PathEscape(PathBuf),
    InvalidRegistry(String),
    Serialization(String),
}

impl fmt::Display for InventoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Context(message) => write!(formatter, "live context is stale: {message}"),
            Self::Io { path, message } => write!(formatter, "{}: {message}", path.display()),
            Self::Json { path, message } => write!(formatter, "{}: {message}", path.display()),
            Self::PathEscape(path) => {
                write!(formatter, "path escapes worktree: {}", path.display())
            }
            Self::InvalidRegistry(message) => write!(formatter, "invalid registry: {message}"),
            Self::Serialization(message) => write!(formatter, "serialization failed: {message}"),
        }
    }
}

impl std::error::Error for InventoryError {}
