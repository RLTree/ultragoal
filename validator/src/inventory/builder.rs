use super::digest::sha256_hex;
use super::types::{
    ActiveStatus, AuthorityCatalog, AuthorityState, GeneratedSurfaceIndex, InventoryEntry,
    InventoryError, MAX_CATALOG_BYTES,
};
use super::{ADOPTED_HANDOFF_DIGEST_CONFIG_KEY, discovery, legacy, registry, routing, validate};
use crate::context::LiveContext;
use serde::Serialize;

pub struct InventoryBuilder<'context> {
    context: &'context LiveContext,
}

#[derive(Serialize)]
struct CatalogIdentity<'a> {
    schema_version: &'static str,
    context_id: &'a str,
    contract_id: &'a str,
    counts: &'a std::collections::BTreeMap<String, usize>,
    entries: &'a [InventoryEntry],
    findings: &'a [super::types::InventoryFinding],
}

impl<'context> InventoryBuilder<'context> {
    pub fn new(context: &'context LiveContext) -> Self {
        Self { context }
    }

    pub fn build(&self) -> Result<AuthorityCatalog, InventoryError> {
        self.context
            .revalidate()
            .map_err(|error| InventoryError::Context(error.to_string()))?;
        let reads = self
            .context
            .begin_read_session()
            .map_err(|error| InventoryError::Context(error.to_string()))?;
        let root = reads.root();
        let canonical_root = root.canonicalize().map_err(|error| InventoryError::Io {
            path: root.to_path_buf(),
            message: error.to_string(),
        })?;
        if canonical_root != root {
            return Err(InventoryError::PathEscape(root.to_path_buf()));
        }
        let expected_handoff_digest = self
            .context
            .configuration()
            .public_values
            .get(ADOPTED_HANDOFF_DIGEST_CONFIG_KEY)
            .ok_or_else(|| {
                InventoryError::InvalidRegistry(
                    "root did not bind the adopted handoff manifest digest".to_owned(),
                )
            })?;
        let registry = registry::load(&reads, root, expected_handoff_digest)?;
        let routing = routing::load(&reads, root, &registry.contract_id)?;
        let discovered = discovery::discover(&reads, root, &registry)?;
        let (legacy_entries, legacy_findings) = legacy::discover(&reads, root)?;
        let context_entry = InventoryEntry {
            stable_id: format!("CONTEXT:{}", self.context.context_id()),
            kind: "live-context".to_owned(),
            owner_role: "OWN-AUTHORITY-KERNEL".to_owned(),
            relative_path: format!("@context/{}", self.context.context_id()),
            digest_sha256: self
                .context
                .context_id()
                .trim_start_matches("sha256:")
                .to_owned(),
            unix_mode: None,
            authority_state: AuthorityState::Context,
            active_status: ActiveStatus::Active,
            generator: Some("HCT-CONTEXT".to_owned()),
            input_provenance: Vec::new(),
            references: Vec::new(),
        };
        let mut findings = registry.findings;
        findings.extend(discovered.findings);
        findings.extend(legacy_findings);
        let routing_entry = routing.registry_entry.clone();
        let (entries, findings) = validate::reconcile(
            root,
            vec![
                registry.entries,
                discovered.entries,
                legacy_entries,
                vec![routing_entry],
                vec![context_entry],
            ],
            findings,
            &routing,
        );
        reads
            .revalidate()
            .map_err(|error| InventoryError::Context(error.to_string()))?;
        let generated = GeneratedSurfaceIndex::new(
            entries
                .iter()
                .filter(|entry| entry.kind == "generated-surface")
                .cloned()
                .collect(),
        );
        let identity = CatalogIdentity {
            schema_version: "AuthorityCatalog-v1",
            context_id: self.context.context_id(),
            contract_id: &registry.contract_id,
            counts: &registry.counts,
            entries: &entries,
            findings: &findings,
        };
        let bytes = serde_json::to_vec(&identity)
            .map_err(|error| InventoryError::Serialization(error.to_string()))?;
        if bytes.len() > MAX_CATALOG_BYTES {
            return Err(InventoryError::Serialization(format!(
                "catalog identity exceeds {MAX_CATALOG_BYTES} bytes"
            )));
        }
        Ok(AuthorityCatalog::new(
            format!("sha256:{}", sha256_hex(&bytes)),
            self.context.context_id().to_owned(),
            registry.contract_id,
            registry.counts,
            entries,
            findings,
            generated,
        ))
    }
}
