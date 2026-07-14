use super::*;

impl PolicyPermit {
    pub(crate) fn issue(
        authority_id: &str,
        claim_registry_id: &str,
        context_id: &str,
        authority_catalog_id: &str,
        candidate_id: &str,
        spec_id: &str,
    ) -> Result<Self, StateError> {
        let mut permit = Self {
            schema_version: "PolicyPermit-v1",
            authority_id: authority_id.to_owned(),
            claim_registry_id: claim_registry_id.to_owned(),
            context_id: context_id.to_owned(),
            authority_catalog_id: authority_catalog_id.to_owned(),
            candidate_id: candidate_id.to_owned(),
            spec_id: spec_id.to_owned(),
            permit_id: String::new(),
        };
        permit.permit_id = permit.identity_id()?;
        Ok(permit)
    }

    pub(crate) fn permit_id(&self) -> &str {
        &self.permit_id
    }

    pub(crate) fn verify_catalog(&self, spec_id: &str, catalog_id: &str) -> Result<(), StateError> {
        if self.spec_id != spec_id || self.permit_id != catalog_id {
            return invalid("policy-permit-catalog-mismatch");
        }
        if self.identity_id()? != self.permit_id {
            return invalid("policy-permit-integrity-mismatch");
        }
        Ok(())
    }

    pub(crate) fn verify_binding(
        &self,
        context_id: &str,
        authority_catalog_id: &str,
        candidate_id: Option<&str>,
    ) -> Result<(), StateError> {
        if self.context_id != context_id {
            return invalid("policy-context-binding-mismatch");
        }
        if self.authority_catalog_id != authority_catalog_id {
            return invalid("policy-authority-catalog-binding-mismatch");
        }
        if candidate_id.is_some_and(|candidate| self.candidate_id != candidate) {
            return invalid("policy-candidate-binding-mismatch");
        }
        Ok(())
    }

    pub(crate) fn identity_id(&self) -> Result<String, StateError> {
        super::super::catalog::policy_digest(&(
            self.schema_version,
            &self.authority_id,
            &self.claim_registry_id,
            &self.context_id,
            &self.authority_catalog_id,
            &self.candidate_id,
            &self.spec_id,
        ))
    }

    #[cfg(test)]
    pub(crate) fn test_mutate(&mut self, field: &str, value: &str) {
        *match field {
            "context" => &mut self.context_id,
            "authority-catalog" => &mut self.authority_catalog_id,
            "candidate" => &mut self.candidate_id,
            "permit-id" => &mut self.permit_id,
            _ => panic!("unknown permit test field"),
        } = value.to_owned();
    }
}

pub(crate) fn verify_live(
    catalog: &DependencyActionCatalog,
    context: &LiveContext,
    authority_catalog: &AuthorityCatalog,
) -> Result<(), StateError> {
    let candidate_id = candidate_identity_id(context)?;
    let observed_codes = authority_catalog
        .findings()
        .iter()
        .map(|finding| finding.code.clone())
        .collect();
    verify_bound(
        catalog,
        context.context_id(),
        authority_catalog.catalog_id(),
        authority_catalog.context_id(),
        Some(&candidate_id),
        &observed_codes,
    )
}

pub(crate) fn verify_bound(
    catalog: &DependencyActionCatalog,
    context_id: &str,
    authority_catalog_id: &str,
    authority_catalog_context_id: &str,
    candidate_id: Option<&str>,
    observed_codes: &BTreeSet<String>,
) -> Result<(), StateError> {
    let permit = catalog
        .permit()
        .ok_or_else(|| StateError::InvalidCatalog("policy-authority-missing".to_owned()))?;
    permit.verify_binding(context_id, authority_catalog_id, candidate_id)?;
    permit.verify_catalog(catalog.spec_id(), catalog.catalog_id())?;
    if catalog.recompute_spec_id()? != catalog.spec_id() {
        return invalid("policy-spec-integrity-mismatch");
    }
    verify_expected_bindings(
        catalog.spec(),
        context_id,
        authority_catalog_id,
        authority_catalog_context_id,
    )?;
    verify_inventory_coverage(catalog.spec(), observed_codes)
}

pub(crate) fn verify_expected_bindings(
    spec: &DependencyActionSpec,
    context_id: &str,
    authority_catalog_id: &str,
    authority_catalog_context_id: &str,
) -> Result<(), StateError> {
    if spec.expected_context_id != context_id || authority_catalog_context_id != context_id {
        return invalid("policy-context-binding-mismatch");
    }
    if spec.expected_authority_catalog_id != authority_catalog_id {
        return invalid("policy-authority-catalog-binding-mismatch");
    }
    Ok(())
}

pub(crate) fn verify_inventory_coverage(
    spec: &DependencyActionSpec,
    observed_codes: &BTreeSet<String>,
) -> Result<(), StateError> {
    let expected_codes = spec
        .inventory_policies
        .iter()
        .map(|policy| policy.code.clone())
        .collect::<BTreeSet<_>>();
    if !observed_codes.is_subset(&expected_codes) {
        return invalid("policy-inventory-impact-missing");
    }
    if !expected_codes.is_subset(observed_codes) {
        return invalid("policy-inventory-impact-extra");
    }
    Ok(())
}

pub(crate) fn candidate_identity_id(context: &LiveContext) -> Result<String, StateError> {
    super::super::catalog::policy_digest(context.candidate())
}

pub(crate) fn invalid<T>(code: &str) -> Result<T, StateError> {
    Err(StateError::InvalidCatalog(code.to_owned()))
}
