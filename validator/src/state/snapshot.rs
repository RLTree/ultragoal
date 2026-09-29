use crate::context::LiveContext;
use crate::inventory::{AuthorityCatalog, FindingSeverity};
use std::collections::BTreeMap;

/// Narrow authority input accepted by the state engine.
///
/// Retained inventory remains an exact-coverage input for compatibility
/// operations. Current product state instead supplies a catalog captured from
/// the three current authority owners and source-agent integrity boundary.
pub(crate) trait StateAuthorityCatalog {
    fn catalog_id(&self) -> &str;
    fn context_id(&self) -> &str;
    fn inventory_findings(&self) -> Vec<InventoryObservation>;
    fn revalidate(&self, context: &LiveContext) -> Result<(), super::product_state::StateError>;
}

impl StateAuthorityCatalog for AuthorityCatalog {
    fn catalog_id(&self) -> &str {
        self.catalog_id()
    }

    fn context_id(&self) -> &str {
        self.context_id()
    }

    fn inventory_findings(&self) -> Vec<InventoryObservation> {
        self.findings()
            .iter()
            .map(|finding| InventoryObservation {
                code: finding.code.clone(),
                severity: finding.severity,
                entry_id: finding.entry_id.clone(),
                relative_path: finding.relative_path.clone(),
                cause: finding.message.clone(),
            })
            .collect()
    }

    fn revalidate(&self, _context: &LiveContext) -> Result<(), super::product_state::StateError> {
        self.revalidate_identity().map_err(|error| {
            super::product_state::StateError::InvalidCatalog(format!(
                "authority-catalog-integrity-invalid:{error}"
            ))
        })
    }
}

/// Current-only authority catalog. It deliberately carries no adopted
/// inventory findings: source capture either validates its exact source
/// boundary or refuses the entire current route.
pub(crate) struct CurrentAuthorityCatalog {
    context_id: String,
    catalog_id: String,
    authority_digest: String,
    source: crate::plugin_product::agent_discovery::CurrentSourceCapture,
}

impl CurrentAuthorityCatalog {
    pub(crate) fn capture(context: &LiveContext) -> Result<Self, super::product_state::StateError> {
        context
            .revalidate()
            .map_err(|error| super::product_state::StateError::StaleContext(error.to_string()))?;
        if context
            .configuration()
            .public_values
            .contains_key("ultragoal.adopted_handoff_manifest_sha256")
        {
            return Err(super::product_state::StateError::InvalidCatalog(
                "current-authority-adopted-handoff-configuration".to_owned(),
            ));
        }
        let candidate_id = super::policy_authority::candidate_identity_id(context)?;
        let authority_digest = crate::product_inception::current_authority_digest(context)
            .map_err(|_| {
                super::product_state::StateError::InvalidCatalog(
                    "current-product-authority-invalid".to_owned(),
                )
            })?;
        let source = crate::plugin_product::agent_discovery::capture_current_source(
            context.worktree_root(),
            &candidate_id,
            context.context_id(),
        )
        .map_err(|_| {
            super::product_state::StateError::InvalidCatalog(
                "current-source-agent-catalog-invalid".to_owned(),
            )
        })?;
        source.revalidate().map_err(|_| {
            super::product_state::StateError::InvalidCatalog(
                "current-source-agent-catalog-changed".to_owned(),
            )
        })?;
        if crate::product_inception::current_authority_digest(context).map_err(|_| {
            super::product_state::StateError::InvalidCatalog(
                "current-product-authority-invalid".to_owned(),
            )
        })? != authority_digest
        {
            return Err(super::product_state::StateError::StaleContext(
                "current product authority changed during catalog capture".to_owned(),
            ));
        }
        context
            .revalidate()
            .map_err(|error| super::product_state::StateError::StaleContext(error.to_string()))?;
        let catalog_id = super::catalog::policy_digest(&(
            "CurrentAuthorityCatalog-v1",
            context.context_id(),
            &candidate_id,
            &authority_digest,
            source.catalog_sha256(),
        ))?;
        Ok(Self {
            context_id: context.context_id().to_owned(),
            catalog_id,
            authority_digest,
            source,
        })
    }
}

impl StateAuthorityCatalog for CurrentAuthorityCatalog {
    fn catalog_id(&self) -> &str {
        &self.catalog_id
    }

    fn context_id(&self) -> &str {
        &self.context_id
    }

    fn inventory_findings(&self) -> Vec<InventoryObservation> {
        Vec::new()
    }

    fn revalidate(&self, context: &LiveContext) -> Result<(), super::product_state::StateError> {
        if context.context_id() != self.context_id {
            return Err(super::product_state::StateError::InvalidCatalog(
                "current-authority-catalog-context-mismatch".to_owned(),
            ));
        }
        self.source.revalidate().map_err(|_| {
            super::product_state::StateError::InvalidCatalog(
                "current-source-agent-catalog-changed".to_owned(),
            )
        })?;
        let authority_digest = crate::product_inception::current_authority_digest(context)
            .map_err(|_| {
                super::product_state::StateError::InvalidCatalog(
                    "current-product-authority-invalid".to_owned(),
                )
            })?;
        if authority_digest != self.authority_digest {
            return Err(super::product_state::StateError::StaleContext(
                "current product authority changed after catalog capture".to_owned(),
            ));
        }
        Ok(())
    }
}

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
    pub candidate_id: String,
    pub inventory_findings: Vec<InventoryObservation>,
    pub capabilities: BTreeMap<String, bool>,
}

impl BoundInputs {
    pub(crate) fn from_live<C: StateAuthorityCatalog + ?Sized>(
        context: &LiveContext,
        catalog: &C,
    ) -> Result<Self, super::product_state::StateError> {
        let mut inventory_findings = catalog.inventory_findings();
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
        Ok(Self {
            context_id: context.context_id().to_owned(),
            authority_catalog_id: catalog.catalog_id().to_owned(),
            authority_catalog_context_id: catalog.context_id().to_owned(),
            candidate_id: super::policy_authority::candidate_identity_id(context)?,
            inventory_findings,
            capabilities,
        })
    }

    pub(crate) fn validate(&self) -> Result<(), super::product_state::StateError> {
        if self.inventory_findings.len() > super::limits::MAX_INVENTORY_FINDINGS {
            return Err(super::product_state::StateError::ResourceLimit(
                "authority catalog finding count".to_owned(),
            ));
        }
        if self.capabilities.len() > super::limits::MAX_CAPABILITIES {
            return Err(super::product_state::StateError::ResourceLimit(
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
            return Err(super::product_state::StateError::ResourceLimit(
                "authority catalog finding bytes".to_owned(),
            ));
        }
        if !super::limits::valid_id(&self.context_id)
            || !super::limits::valid_id(&self.authority_catalog_id)
            || !super::limits::valid_id(&self.authority_catalog_context_id)
            || !super::limits::valid_id(&self.candidate_id)
        {
            return Err(super::product_state::StateError::InvalidCatalog(
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
                return Err(super::product_state::StateError::InvalidCatalog(
                    "unsafe authority catalog finding".to_owned(),
                ));
            }
        }
        if self
            .capabilities
            .keys()
            .any(|name| !super::limits::valid_id(name))
        {
            return Err(super::product_state::StateError::InvalidCatalog(
                "invalid capability identity".to_owned(),
            ));
        }
        Ok(())
    }
}
