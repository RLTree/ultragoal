use super::model::{
    MAX_COLLECTION, validate_actor_identifier, validate_digest, validate_identifier,
};
use super::{Binding, CanonicalPath, OrchestrationError};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegrationDisposition {
    None,
    Full,
    Partial,
    Conflict,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RootIntegrationIntent {
    pub schema_version: String,
    pub base_binding: Binding,
    pub root_actor: String,
    pub accepted_proposals: BTreeMap<String, String>,
    pub prior_digests: BTreeMap<String, Option<String>>,
    pub expected_digests: BTreeMap<String, String>,
}

impl RootIntegrationIntent {
    pub fn intent_id(&self) -> Result<String, OrchestrationError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self).map_err(|_| OrchestrationError::InvalidReview)?;
        Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
    }

    pub(super) fn observe(
        &self,
        observed_digests: BTreeMap<String, Option<String>>,
        evidence_digest: &str,
    ) -> Result<RootIntegrationObservation, OrchestrationError> {
        validate_digest(evidence_digest)?;
        let disposition = self.classify(&observed_digests)?;
        let observed_set_sha256 = observed_set_sha256(&observed_digests)?;
        Ok(RootIntegrationObservation {
            schema_version: "RootIntegrationObservation-v1".to_owned(),
            intent_id: self.intent_id()?,
            observed_digests,
            observed_set_sha256,
            evidence_digest: evidence_digest.to_owned(),
            disposition,
            live_observed: true,
        })
    }

    pub(crate) fn validate(&self) -> Result<(), OrchestrationError> {
        if self.schema_version != "RootIntegrationIntent-v1"
            || self.accepted_proposals.is_empty()
            || self.accepted_proposals.len() > MAX_COLLECTION
        {
            return Err(OrchestrationError::InvalidReview);
        }
        self.base_binding.validate()?;
        validate_actor_identifier(&self.root_actor)?;
        for (lease_id, proposal_id) in &self.accepted_proposals {
            validate_identifier(lease_id)?;
            validate_digest(proposal_id)?;
        }
        super::validate_root_change_map(&self.expected_digests)?;
        validate_observed_map(&self.prior_digests)?;
        if self.prior_digests.len() != self.expected_digests.len()
            || !self.prior_digests.keys().eq(self.expected_digests.keys())
            || self.prior_digests.iter().any(|(path, prior)| {
                prior.as_ref().is_some_and(|digest| {
                    self.expected_digests
                        .get(path)
                        .is_some_and(|next| next == digest)
                })
            })
        {
            return Err(OrchestrationError::InvalidReview);
        }
        Ok(())
    }

    fn classify(
        &self,
        observed: &BTreeMap<String, Option<String>>,
    ) -> Result<IntegrationDisposition, OrchestrationError> {
        self.validate()?;
        validate_observed_map(observed)?;
        if observed.len() != self.prior_digests.len()
            || !observed.keys().eq(self.prior_digests.keys())
        {
            return Err(OrchestrationError::IntegrationAmbiguous);
        }
        let all_prior = observed == &self.prior_digests;
        let all_expected = observed
            .iter()
            .all(|(path, value)| value.as_ref() == self.expected_digests.get(path));
        if all_expected {
            return Ok(IntegrationDisposition::Full);
        }
        if all_prior {
            return Ok(IntegrationDisposition::None);
        }
        let only_known = observed.iter().all(|(path, value)| {
            value == self.prior_digests.get(path).expect("equal key sets")
                || value.as_ref() == self.expected_digests.get(path)
        });
        Ok(if only_known {
            IntegrationDisposition::Partial
        } else {
            IntegrationDisposition::Conflict
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RootIntegrationObservation {
    pub(crate) schema_version: String,
    pub(crate) intent_id: String,
    pub(crate) observed_digests: BTreeMap<String, Option<String>>,
    pub(crate) observed_set_sha256: String,
    pub(crate) evidence_digest: String,
    pub(crate) disposition: IntegrationDisposition,
    #[serde(skip)]
    live_observed: bool,
}

impl PartialEq for RootIntegrationObservation {
    fn eq(&self, other: &Self) -> bool {
        self.schema_version == other.schema_version
            && self.intent_id == other.intent_id
            && self.observed_digests == other.observed_digests
            && self.observed_set_sha256 == other.observed_set_sha256
            && self.evidence_digest == other.evidence_digest
            && self.disposition == other.disposition
    }
}

impl Eq for RootIntegrationObservation {}

impl RootIntegrationObservation {
    pub fn disposition(&self) -> IntegrationDisposition {
        self.disposition
    }

    pub fn observed_set_sha256(&self) -> &str {
        &self.observed_set_sha256
    }

    pub(crate) fn validate_for(
        &self,
        intent: &RootIntegrationIntent,
        persisted_replay: bool,
    ) -> Result<(), OrchestrationError> {
        if self.schema_version != "RootIntegrationObservation-v1"
            || (!self.live_observed && !persisted_replay)
            || self.intent_id != intent.intent_id()?
            || self.observed_set_sha256 != observed_set_sha256(&self.observed_digests)?
            || self.disposition != intent.classify(&self.observed_digests)?
        {
            return Err(OrchestrationError::IntegrationAmbiguous);
        }
        validate_digest(&self.evidence_digest)
    }
}

fn observed_set_sha256(
    map: &BTreeMap<String, Option<String>>,
) -> Result<String, OrchestrationError> {
    validate_observed_map(map)?;
    let bytes = serde_json::to_vec(map).map_err(|_| OrchestrationError::IntegrationAmbiguous)?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn validate_observed_map(map: &BTreeMap<String, Option<String>>) -> Result<(), OrchestrationError> {
    if map.len() > MAX_COLLECTION {
        return Err(OrchestrationError::ResourceLimit);
    }
    let mut paths = Vec::with_capacity(map.len());
    for (raw, digest) in map {
        let path = CanonicalPath::parse(raw)?;
        if let Some(digest) = digest {
            validate_digest(digest)?;
        }
        if paths
            .iter()
            .any(|prior: &CanonicalPath| prior.overlaps(&path))
        {
            return Err(OrchestrationError::DuplicateOutput);
        }
        paths.push(path);
    }
    Ok(())
}
