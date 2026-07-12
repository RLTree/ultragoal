use super::catalog::{ClaimSpec, DependencyActionCatalog, DependencyActionSpec};
use super::types::StateError;
use crate::context::LiveContext;
use crate::inventory::AuthorityCatalog;
use serde::Serialize;
use std::collections::BTreeSet;

/// Root-owned bridge from an adopted claim-registry loader to state derivation.
///
/// The state module does not load or duplicate the registry. Root integration
/// supplies its verified digest, exact typed claims, and one accepted live policy.
#[allow(dead_code)]
pub(crate) struct PolicyAuthority {
    claim_registry_id: String,
    authority_id: String,
    adopted_claims: Vec<ClaimSpec>,
    accepted_catalog: DependencyActionCatalog,
}

#[allow(dead_code)]
impl PolicyAuthority {
    pub(crate) fn from_adopted_claim_registry(
        claim_registry_id: impl Into<String>,
        adopted_claims: Vec<ClaimSpec>,
        accepted_spec: DependencyActionSpec,
    ) -> Result<Self, StateError> {
        let claim_registry_id = claim_registry_id.into();
        if !super::catalog::valid_sha256(&claim_registry_id) {
            return invalid("policy-claim-registry-identity-invalid");
        }
        super::catalog::validate_inventory_impacts(&accepted_spec.inventory_policies)?;
        let adopted_claims = super::catalog::canonical_claims(adopted_claims)?;
        let accepted_catalog = DependencyActionCatalog::from_untrusted_spec(accepted_spec)?;
        if accepted_catalog.spec().claims != adopted_claims {
            return invalid("policy-claim-set-mismatch");
        }
        let authority_id = super::catalog::policy_digest(&(
            "PolicyAuthority-v1",
            &claim_registry_id,
            &adopted_claims,
            accepted_catalog.spec_id(),
        ))?;
        Ok(Self {
            claim_registry_id,
            authority_id,
            adopted_claims,
            accepted_catalog,
        })
    }

    pub(crate) fn issue(
        &self,
        context: &LiveContext,
        authority_catalog: &AuthorityCatalog,
    ) -> Result<DependencyActionCatalog, StateError> {
        context
            .revalidate()
            .map_err(|error| StateError::StaleContext(error.to_string()))?;
        let candidate_id = candidate_identity_id(context)?;
        let observed_codes = authority_catalog
            .findings()
            .iter()
            .map(|finding| finding.code.clone())
            .collect();
        let catalog = self.issue_bound(
            context.context_id(),
            authority_catalog.catalog_id(),
            authority_catalog.context_id(),
            &candidate_id,
            &observed_codes,
        )?;
        context
            .revalidate()
            .map_err(|error| StateError::StaleContext(error.to_string()))?;
        verify_live(&catalog, context, authority_catalog)?;
        Ok(catalog)
    }

    fn issue_bound(
        &self,
        context_id: &str,
        authority_catalog_id: &str,
        authority_catalog_context_id: &str,
        candidate_id: &str,
        observed_codes: &BTreeSet<String>,
    ) -> Result<DependencyActionCatalog, StateError> {
        verify_expected_bindings(
            self.accepted_catalog.spec(),
            context_id,
            authority_catalog_id,
            authority_catalog_context_id,
        )?;
        verify_inventory_coverage(self.accepted_catalog.spec(), observed_codes)?;
        let permit = PolicyPermit::issue(
            &self.authority_id,
            &self.claim_registry_id,
            context_id,
            authority_catalog_id,
            candidate_id,
            self.accepted_catalog.spec_id(),
        )?;
        self.accepted_catalog.clone().authorize(permit)
    }

    #[cfg(test)]
    pub(crate) fn issue_for_test(
        &self,
        context_id: &str,
        authority_catalog_id: &str,
        candidate_id: &str,
        observed_codes: &BTreeSet<String>,
    ) -> Result<DependencyActionCatalog, StateError> {
        self.issue_bound(
            context_id,
            authority_catalog_id,
            context_id,
            candidate_id,
            observed_codes,
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct PolicyPermit {
    schema_version: &'static str,
    authority_id: String,
    claim_registry_id: String,
    context_id: String,
    authority_catalog_id: String,
    candidate_id: String,
    spec_id: String,
    permit_id: String,
}

impl PolicyPermit {
    fn issue(
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

    pub(super) fn permit_id(&self) -> &str {
        &self.permit_id
    }

    pub(super) fn verify_catalog(&self, spec_id: &str, catalog_id: &str) -> Result<(), StateError> {
        if self.spec_id != spec_id || self.permit_id != catalog_id {
            return invalid("policy-permit-catalog-mismatch");
        }
        if self.identity_id()? != self.permit_id {
            return invalid("policy-permit-integrity-mismatch");
        }
        Ok(())
    }

    fn verify_binding(
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

    fn identity_id(&self) -> Result<String, StateError> {
        super::catalog::policy_digest(&(
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

pub(super) fn verify_bound(
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

fn verify_expected_bindings(
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

fn verify_inventory_coverage(
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

fn candidate_identity_id(context: &LiveContext) -> Result<String, StateError> {
    super::catalog::policy_digest(context.candidate())
}

fn invalid<T>(code: &str) -> Result<T, StateError> {
    Err(StateError::InvalidCatalog(code.to_owned()))
}
