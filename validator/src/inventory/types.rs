use super::digest::sha256_hex;
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
    Candidate,
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

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum InventoryClosureState {
    Closed,
    Blocked,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InventoryFindingDisposition {
    Blocking,
    OpenMigrationObligation,
    Informational,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct InventoryClosureStatus {
    schema_version: &'static str,
    catalog_id: String,
    context_id: String,
    state: InventoryClosureState,
    blocker_count: usize,
    blockers_by_code: BTreeMap<String, usize>,
    open_obligation_count: usize,
    open_obligations_by_code: BTreeMap<String, usize>,
}

impl InventoryClosureStatus {
    pub fn state(&self) -> InventoryClosureState {
        self.state
    }

    pub fn is_closed(&self) -> bool {
        self.state == InventoryClosureState::Closed
    }

    pub fn blocker_count(&self) -> usize {
        self.blocker_count
    }

    pub fn blockers_by_code(&self) -> &BTreeMap<String, usize> {
        &self.blockers_by_code
    }

    pub fn open_obligation_count(&self) -> usize {
        self.open_obligation_count
    }

    pub fn open_obligations_by_code(&self) -> &BTreeMap<String, usize> {
        &self.open_obligations_by_code
    }

    pub(crate) fn new(
        catalog_id: String,
        context_id: String,
        blockers_by_code: BTreeMap<String, usize>,
        open_obligations_by_code: BTreeMap<String, usize>,
    ) -> Self {
        let blocker_count = blockers_by_code.values().sum();
        let open_obligation_count = open_obligations_by_code.values().sum();
        Self {
            schema_version: "InventoryClosureStatus-v2",
            catalog_id,
            context_id,
            state: if blocker_count == 0 {
                InventoryClosureState::Closed
            } else {
                InventoryClosureState::Blocked
            },
            blocker_count,
            blockers_by_code,
            open_obligation_count,
            open_obligations_by_code,
        }
    }
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
    pub(crate) fn closure_disposition(&self) -> InventoryFindingDisposition {
        match self.severity {
            FindingSeverity::Error => InventoryFindingDisposition::Blocking,
            FindingSeverity::Warning
                if matches!(
                    self.code.as_str(),
                    "sole_current_authority_pending_migration" | "compatibility_route_retained"
                ) =>
            {
                InventoryFindingDisposition::OpenMigrationObligation
            }
            FindingSeverity::Warning => InventoryFindingDisposition::Blocking,
            FindingSeverity::Info => InventoryFindingDisposition::Informational,
        }
    }

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

#[derive(Serialize)]
struct CatalogIdentity<'a> {
    schema_version: &'static str,
    context_id: &'a str,
    contract_id: &'a str,
    counts: &'a BTreeMap<String, usize>,
    entries: &'a [InventoryEntry],
    findings: &'a [InventoryFinding],
}

pub(crate) fn catalog_identity_id(
    context_id: &str,
    contract_id: &str,
    counts: &BTreeMap<String, usize>,
    entries: &[InventoryEntry],
    findings: &[InventoryFinding],
) -> Result<String, InventoryError> {
    let bytes = serde_json::to_vec(&CatalogIdentity {
        schema_version: "AuthorityCatalog-v1",
        context_id,
        contract_id,
        counts,
        entries,
        findings,
    })
    .map_err(|error| InventoryError::Serialization(error.to_string()))?;
    if bytes.len() > MAX_CATALOG_BYTES {
        return Err(InventoryError::Serialization(format!(
            "catalog identity exceeds {MAX_CATALOG_BYTES} bytes"
        )));
    }
    Ok(format!("sha256:{}", sha256_hex(&bytes)))
}

impl AuthorityCatalog {
    #[cfg(test)]
    pub(crate) fn canonical_for_test(
        context_id: String,
        contract_id: String,
        source_registry_counts: BTreeMap<String, usize>,
        entries: Vec<InventoryEntry>,
        findings: Vec<InventoryFinding>,
    ) -> Result<Self, InventoryError> {
        let generated_surfaces = GeneratedSurfaceIndex::new(
            entries
                .iter()
                .filter(|entry| entry.kind == "generated-surface")
                .cloned()
                .collect(),
        );
        let catalog_id = catalog_identity_id(
            &context_id,
            &contract_id,
            &source_registry_counts,
            &entries,
            &findings,
        )?;
        Ok(Self::new(
            catalog_id,
            context_id,
            contract_id,
            source_registry_counts,
            entries,
            findings,
            generated_surfaces,
        ))
    }

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

    pub(crate) fn revalidate_identity(&self) -> Result<(), InventoryError> {
        let expected_generated = self
            .entries
            .iter()
            .filter(|entry| entry.kind == "generated-surface")
            .cloned()
            .collect::<Vec<_>>();
        if self.generated_surfaces.entries() != expected_generated {
            return Err(InventoryError::InvalidRegistry(
                "authority catalog generated surfaces are not the canonical entry projection"
                    .to_owned(),
            ));
        }
        let current = catalog_identity_id(
            &self.context_id,
            &self.contract_id,
            &self.source_registry_counts,
            &self.entries,
            &self.findings,
        )?;
        if current != self.catalog_id {
            return Err(InventoryError::InvalidRegistry(
                "authority catalog identity does not match its canonical contents".to_owned(),
            ));
        }
        Ok(())
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
    pub input_verified_in_session: bool,
    pub authority_eligible: bool,
    pub expected_sha256: String,
    pub actual_sha256: String,
    pub findings: Vec<InventoryFinding>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActivationFailure {
    UnknownRow,
    MissingRow,
    DuplicateRow,
    ConflictingRow,
    StaleInput,
    UnsafeInput,
}

impl ActivationFailure {
    pub const fn code(self) -> &'static str {
        match self {
            Self::UnknownRow => "unknown-row",
            Self::MissingRow => "missing-row",
            Self::DuplicateRow => "duplicate-row",
            Self::ConflictingRow => "conflicting-row",
            Self::StaleInput => "stale-input",
            Self::UnsafeInput => "unsafe-input",
        }
    }
}

#[derive(Debug)]
pub enum InventoryError {
    Context(String),
    Io { path: PathBuf, message: String },
    Json { path: PathBuf, message: String },
    PathEscape(PathBuf),
    InvalidRegistry(String),
    Activation(ActivationFailure),
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
            Self::Activation(failure) => {
                write!(formatter, "inventory activation failed: {}", failure.code())
            }
            Self::Serialization(message) => write!(formatter, "serialization failed: {message}"),
        }
    }
}

impl std::error::Error for InventoryError {}

#[cfg(test)]
mod catalog_identity_tests {
    use super::*;

    fn catalog(catalog_id: String, generated_surfaces: GeneratedSurfaceIndex) -> AuthorityCatalog {
        AuthorityCatalog::new(
            catalog_id,
            "sha256:context".to_owned(),
            "test-contract".to_owned(),
            BTreeMap::new(),
            Vec::new(),
            Vec::new(),
            generated_surfaces,
        )
    }

    #[test]
    fn one_canonical_identity_helper_issues_and_revalidates_catalogs() {
        let id = catalog_identity_id(
            "sha256:context",
            "test-contract",
            &BTreeMap::new(),
            &[],
            &[],
        )
        .expect("catalog identity");
        let catalog = AuthorityCatalog::canonical_for_test(
            "sha256:context".to_owned(),
            "test-contract".to_owned(),
            BTreeMap::new(),
            Vec::new(),
            Vec::new(),
        )
        .expect("canonical test catalog");
        assert_eq!(catalog.catalog_id(), id);
        catalog.revalidate_identity().expect("canonical catalog");
    }

    #[test]
    fn forged_id_and_noncanonical_generated_projection_fail_closed() {
        assert!(
            catalog(
                format!("sha256:{}", "1".repeat(64)),
                GeneratedSurfaceIndex::new(Vec::new()),
            )
            .revalidate_identity()
            .is_err()
        );

        let id = catalog_identity_id(
            "sha256:context",
            "test-contract",
            &BTreeMap::new(),
            &[],
            &[],
        )
        .expect("catalog identity");
        let forged_projection = InventoryEntry {
            stable_id: "GEN:forged".to_owned(),
            kind: "generated-surface".to_owned(),
            owner_role: "OWN-TEST".to_owned(),
            relative_path: "generated/forged.json".to_owned(),
            digest_sha256: "0".repeat(64),
            unix_mode: None,
            authority_state: AuthorityState::Projection,
            active_status: ActiveStatus::Candidate,
            generator: Some("test".to_owned()),
            input_provenance: Vec::new(),
            references: Vec::new(),
        };
        assert!(
            catalog(id, GeneratedSurfaceIndex::new(vec![forged_projection]),)
                .revalidate_identity()
                .is_err()
        );
    }
}
