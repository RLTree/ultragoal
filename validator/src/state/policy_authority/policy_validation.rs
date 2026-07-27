use super::*;

/// Root-owned bridge from an adopted claim-registry loader to state derivation.
///
/// The state module does not load or duplicate the registry. Root integration
/// supplies its verified digest, exact typed claims, and one accepted live policy.
pub(crate) struct PolicyAuthority {
    claim_registry_id: String,
    authority_id: String,
    accepted_catalog: DependencyActionCatalog,
}

impl PolicyAuthority {
    pub(crate) fn seal_verification_mode(
        &self,
        proposal: crate::engineering_advisory::VerificationModeProposal,
    ) -> Result<crate::engineering_advisory::VerificationModeContract, StateError> {
        crate::engineering_advisory::VerificationModeContract::seal_from_root(
            proposal,
            &self.authority_id,
        )
        .map_err(|_| StateError::InvalidCatalog("verification-mode-root-seal-failed".to_owned()))
    }

    pub(crate) fn from_adopted_claim_registry(
        claim_registry_id: impl Into<String>,
        adopted_claims: Vec<ClaimSpec>,
        accepted_spec: DependencyActionSpec,
    ) -> Result<Self, StateError> {
        let claim_registry_id = claim_registry_id.into();
        if !super::super::catalog::valid_sha256(&claim_registry_id) {
            return invalid("policy-claim-registry-identity-invalid");
        }
        super::super::catalog::validate_inventory_impacts(&accepted_spec.inventory_policies)?;
        let adopted_claims = super::super::catalog::canonical_claims(adopted_claims)?;
        let accepted_catalog = DependencyActionCatalog::from_untrusted_spec(accepted_spec)?;
        if accepted_catalog.spec().claims != adopted_claims {
            return invalid("policy-claim-set-mismatch");
        }
        let authority_id = super::super::catalog::policy_digest(&(
            "PolicyAuthority-v1",
            &claim_registry_id,
            &adopted_claims,
            accepted_catalog.spec_id(),
        ))?;
        Ok(Self {
            claim_registry_id,
            authority_id,
            accepted_catalog,
        })
    }

    pub(crate) fn issue(
        &self,
        context: &LiveContext,
        authority_catalog: &AuthorityCatalog,
    ) -> Result<DependencyActionCatalog, StateError> {
        self.issue_with_verification_mode(context, authority_catalog, None)
    }

    pub(crate) fn issue_with_verification_mode(
        &self,
        context: &LiveContext,
        authority_catalog: &AuthorityCatalog,
        proposed_mode: Option<(
            String,
            crate::engineering_advisory::VerificationModeProposal,
        )>,
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
            proposed_mode,
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
        proposed_mode: Option<(
            String,
            crate::engineering_advisory::VerificationModeProposal,
        )>,
    ) -> Result<DependencyActionCatalog, StateError> {
        if let Some((_, proposal)) = proposed_mode.as_ref() {
            validate_verification_candidate(proposal, candidate_id)?;
        }
        let accepted_catalog = if let Some((action_id, proposal)) = proposed_mode {
            let contract = self.seal_verification_mode(proposal)?;
            self.accepted_catalog
                .clone()
                .with_verification_mode(action_id, contract)?
        } else {
            self.accepted_catalog.clone()
        };
        verify_expected_bindings(
            accepted_catalog.spec(),
            context_id,
            authority_catalog_id,
            authority_catalog_context_id,
        )?;
        verify_inventory_coverage(accepted_catalog.spec(), observed_codes)?;
        let permit = PolicyPermit::issue(
            &self.authority_id,
            &self.claim_registry_id,
            context_id,
            authority_catalog_id,
            candidate_id,
            accepted_catalog.spec_id(),
        )?;
        accepted_catalog.authorize(permit)
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
            None,
        )
    }
}

fn validate_verification_candidate(
    proposal: &crate::engineering_advisory::VerificationModeProposal,
    candidate_id: &str,
) -> Result<(), StateError> {
    if proposal.candidate_id != candidate_id {
        return Err(StateError::InvalidCatalog(
            "verification-mode-candidate-mismatch".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_verification_candidate;
    use crate::engineering_advisory::{VerificationModeProposal, VerificationModeProposalInput};
    use crate::state::product_state::StateError;
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn cross_candidate_verification_is_rejected_before_root_sealing() {
        let proposal = VerificationModeProposal::new(VerificationModeProposalInput {
            candidate_id: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .to_owned(),
            risk: "high".to_owned(),
            failure_model: "semantic mutation".to_owned(),
            oracle: "targeted oracle".to_owned(),
            truth_surface: "source".to_owned(),
            selected_modes: BTreeSet::from(["targeted".to_owned()]),
            rejected_modes: BTreeSet::new(),
            required_evidence: BTreeSet::from(["semantic-check".to_owned()]),
            claim_ceiling: BTreeMap::from([(
                "CL-SOURCE".to_owned(),
                BTreeSet::from(["source".to_owned()]),
            )]),
            invalidation_trigger: "candidate changed".to_owned(),
        })
        .unwrap();
        assert_eq!(
            validate_verification_candidate(
                &proposal,
                "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            ),
            Err(StateError::InvalidCatalog(
                "verification-mode-candidate-mismatch".to_owned()
            ))
        );
        assert!(validate_verification_candidate(&proposal, &proposal.candidate_id).is_ok());
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct PolicyPermit {
    pub(crate) schema_version: &'static str,
    pub(crate) authority_id: String,
    pub(crate) claim_registry_id: String,
    pub(crate) context_id: String,
    pub(crate) authority_catalog_id: String,
    pub(crate) candidate_id: String,
    pub(crate) spec_id: String,
    pub(crate) permit_id: String,
}
