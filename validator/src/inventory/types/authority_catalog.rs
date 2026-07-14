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

pub(crate) struct AuthorityCatalogDefinition {
    pub catalog_id: String,
    pub context_id: String,
    pub contract_id: String,
    pub source_registry_counts: BTreeMap<String, usize>,
    pub entries: Vec<InventoryEntry>,
    pub findings: Vec<InventoryFinding>,
    pub generated_surfaces: GeneratedSurfaceIndex,
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
        Ok(Self::new(AuthorityCatalogDefinition {
            catalog_id,
            context_id,
            contract_id,
            source_registry_counts,
            entries,
            findings,
            generated_surfaces,
        }))
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

    pub fn revalidate_identity(&self) -> Result<(), InventoryError> {
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

    pub(crate) fn new(definition: AuthorityCatalogDefinition) -> Self {
        let AuthorityCatalogDefinition {
            catalog_id,
            context_id,
            contract_id,
            source_registry_counts,
            entries,
            findings,
            generated_surfaces,
        } = definition;
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
