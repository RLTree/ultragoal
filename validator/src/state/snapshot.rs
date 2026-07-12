use crate::context::LiveContext;
use crate::inventory::{AuthorityCatalog, FindingSeverity};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct InventoryObservation {
    pub code: String,
    pub severity: FindingSeverity,
    pub entry_id: Option<String>,
    pub relative_path: Option<String>,
    pub cause: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BoundInputs {
    pub context_id: String,
    pub authority_catalog_id: String,
    pub authority_catalog_context_id: String,
    pub inventory_findings: Vec<InventoryObservation>,
    pub capabilities: BTreeMap<String, bool>,
}

impl BoundInputs {
    pub(crate) fn from_live(context: &LiveContext, catalog: &AuthorityCatalog) -> Self {
        let mut inventory_findings = catalog
            .findings()
            .iter()
            .map(|finding| InventoryObservation {
                code: finding.code.clone(),
                severity: finding.severity,
                entry_id: finding.entry_id.clone(),
                relative_path: finding.relative_path.clone(),
                cause: finding.message.clone(),
            })
            .collect::<Vec<_>>();
        inventory_findings.sort_by(|a, b| {
            (&a.code, &a.entry_id, &a.relative_path, &a.cause).cmp(&(
                &b.code,
                &b.entry_id,
                &b.relative_path,
                &b.cause,
            ))
        });
        let capabilities = context
            .capabilities()
            .tools
            .iter()
            .map(|tool| (tool.name.clone(), tool.available))
            .collect();
        Self {
            context_id: context.context_id().to_owned(),
            authority_catalog_id: catalog.catalog_id().to_owned(),
            authority_catalog_context_id: catalog.context_id().to_owned(),
            inventory_findings,
            capabilities,
        }
    }

    pub(crate) fn validate(&self) -> Result<(), super::types::StateError> {
        if self.inventory_findings.len() > super::limits::MAX_INVENTORY_FINDINGS {
            return Err(super::types::StateError::ResourceLimit(
                "authority catalog finding count".to_owned(),
            ));
        }
        if self.capabilities.len() > super::limits::MAX_CAPABILITIES {
            return Err(super::types::StateError::ResourceLimit(
                "live capability count".to_owned(),
            ));
        }
        let observed_bytes = self
            .inventory_findings
            .iter()
            .map(|finding| {
                finding.code.len()
                    + finding.cause.len()
                    + finding.entry_id.as_deref().map_or(0, str::len)
                    + finding.relative_path.as_deref().map_or(0, str::len)
            })
            .sum::<usize>();
        if observed_bytes > super::limits::MAX_PROJECTION_BYTES {
            return Err(super::types::StateError::ResourceLimit(
                "authority catalog finding bytes".to_owned(),
            ));
        }
        if !super::limits::valid_id(&self.context_id)
            || !super::limits::valid_id(&self.authority_catalog_id)
            || !super::limits::valid_id(&self.authority_catalog_context_id)
        {
            return Err(super::types::StateError::InvalidCatalog(
                "invalid bound identity".to_owned(),
            ));
        }
        for finding in &self.inventory_findings {
            if !super::limits::valid_id(&finding.code)
                || !super::limits::valid_text(&finding.cause, false)
                || finding
                    .entry_id
                    .as_deref()
                    .is_some_and(|value| !super::limits::valid_text(value, false))
                || finding
                    .relative_path
                    .as_deref()
                    .is_some_and(|value| !super::limits::valid_relative_path(value))
            {
                return Err(super::types::StateError::InvalidCatalog(
                    "unsafe authority catalog finding".to_owned(),
                ));
            }
        }
        if self
            .capabilities
            .keys()
            .any(|name| !super::limits::valid_id(name))
        {
            return Err(super::types::StateError::InvalidCatalog(
                "invalid capability identity".to_owned(),
            ));
        }
        Ok(())
    }
}
