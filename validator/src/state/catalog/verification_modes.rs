use super::super::product_state::StateError;
use super::fact_catalog::DependencyActionCatalog;
use crate::engineering_advisory::VerificationModeContract;
use sha2::Digest;
use std::collections::BTreeMap;

impl DependencyActionCatalog {
    pub(crate) fn verification_modes(&self) -> &BTreeMap<String, VerificationModeContract> {
        &self.verification_modes
    }

    /// Attach a root-issued contract to one existing evidence-led action
    /// before authority issuance. The binding is part of catalog identity.
    pub(crate) fn with_verification_mode(
        mut self,
        action_id: impl Into<String>,
        contract: VerificationModeContract,
    ) -> Result<Self, StateError> {
        if self.authority.is_some() {
            return Err(StateError::InvalidCatalog(
                "verification-mode-after-authorization".to_owned(),
            ));
        }
        let action_id = action_id.into();
        let Some(action) = self
            .spec
            .actions
            .iter()
            .find(|action| action.action_id == action_id)
        else {
            return Err(StateError::InvalidCatalog(
                "verification-mode-unknown-action".to_owned(),
            ));
        };
        if action.evidence_led.is_none() {
            return Err(StateError::InvalidCatalog(
                "verification-mode-action-not-evidence-led".to_owned(),
            ));
        }
        contract
            .validate()
            .map_err(|_| StateError::InvalidCatalog("verification-mode-invalid".to_owned()))?;
        if self
            .verification_modes
            .insert(action_id, contract)
            .is_some()
        {
            return Err(StateError::InvalidCatalog(
                "verification-mode-duplicate".to_owned(),
            ));
        }
        self.spec_id = self.recompute_spec_id()?;
        self.catalog_id = self.spec_id.clone();
        Ok(self)
    }

    pub(crate) fn recompute_spec_id(&self) -> Result<String, StateError> {
        let bytes = serde_json::to_vec(&(
            super::claim_catalog::CatalogIdentity::from(&self.spec),
            &self.verification_modes,
        ))
        .map_err(|error| StateError::Serialization(error.to_string()))?;
        Ok(format!("sha256:{:x}", sha2::Sha256::digest(bytes)))
    }
}
