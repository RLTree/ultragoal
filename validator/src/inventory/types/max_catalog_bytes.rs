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
                    "sole_current_authority_pending_migration"
                        | "compatibility_route_retained"
                        | "candidate_component_not_active"
                        | "projection_requires_canonical_reconciliation"
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
