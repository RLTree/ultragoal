use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use crate::distribution::json;
use crate::distribution::spec::digest;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug)]
pub struct ProvenanceExpectation {
    pub context_id: String,
    pub candidate_id: String,
    pub subject_name: String,
    pub subject_sha256: String,
    pub builder_id: String,
    pub predicate_type: String,
    pub materials: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProvenanceSnapshot {
    context_id: String,
    candidate_id: String,
    subject_sha256: String,
    material_count: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Provenance {
    schema: String,
    context_id: String,
    candidate_id: String,
    subject: Subject,
    builder_id: String,
    predicate_type: String,
    materials: Vec<Material>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Subject {
    name: String,
    sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Material {
    uri: String,
    sha256: String,
}

pub fn verify_provenance(
    bytes: &[u8],
    expected: &ProvenanceExpectation,
) -> Result<ProvenanceSnapshot, DistributionError> {
    if !valid_provenance_expectation(expected) {
        return Err(error(DistributionErrorId::InvalidSpec));
    }
    let value: Provenance = json::parse(bytes, 1024 * 1024)?;
    let mut materials = BTreeMap::new();
    for row in value.materials {
        if row.uri.is_empty()
            || row.uri.len() > 512
            || !digest(&row.sha256)
            || materials.insert(row.uri, row.sha256).is_some()
        {
            return Err(error(DistributionErrorId::ProvenanceMismatch));
        }
    }
    if value.schema != "harness-ultragoal.provenance.v1"
        || value.context_id != expected.context_id
        || value.candidate_id != expected.candidate_id
        || value.subject.name != expected.subject_name
        || value.subject.sha256 != expected.subject_sha256
        || value.builder_id != expected.builder_id
        || value.predicate_type != expected.predicate_type
        || materials != expected.materials
    {
        return Err(error(DistributionErrorId::ProvenanceMismatch));
    }
    Ok(ProvenanceSnapshot {
        context_id: expected.context_id.clone(),
        candidate_id: expected.candidate_id.clone(),
        subject_sha256: expected.subject_sha256.clone(),
        material_count: materials.len(),
    })
}

#[derive(Clone, Debug)]
pub struct SignatureExpectation {
    pub identity: String,
    pub issuer: String,
    pub key_id: String,
    pub subject_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SignatureSnapshot {
    subject_sha256: String,
    expectation_bound: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SignatureEnvelope {
    schema: String,
    identity: String,
    issuer: String,
    key_id: String,
    subject_sha256: String,
    signature: String,
}

pub trait SignatureVerifierEffects {
    fn verify_signature(&mut self, envelope: &[u8], subject_sha256: &str) -> Result<bool, ()>;
}

pub fn verify_signature(
    bytes: &[u8],
    expected: Option<&SignatureExpectation>,
    effects: &mut impl SignatureVerifierEffects,
) -> Result<SignatureSnapshot, DistributionError> {
    let expected =
        expected.ok_or_else(|| error(DistributionErrorId::SignaturePolicyUnavailable))?;
    if expected.identity.is_empty()
        || expected.issuer.is_empty()
        || expected.key_id.is_empty()
        || !digest(&expected.subject_sha256)
    {
        return Err(error(DistributionErrorId::InvalidSpec));
    }
    let envelope: SignatureEnvelope = json::parse(bytes, 1024 * 1024)?;
    if envelope.schema != "harness-ultragoal.signature-envelope.v1"
        || envelope.identity != expected.identity
        || envelope.issuer != expected.issuer
        || envelope.key_id != expected.key_id
        || envelope.subject_sha256 != expected.subject_sha256
        || envelope.signature.is_empty()
        || envelope.signature.len() > 128 * 1024
    {
        return Err(error(DistributionErrorId::SignatureMismatch));
    }
    if !effects
        .verify_signature(bytes, &expected.subject_sha256)
        .map_err(|_| error(DistributionErrorId::EffectFailed))?
    {
        return Err(error(DistributionErrorId::SignatureMismatch));
    }
    Ok(SignatureSnapshot {
        subject_sha256: expected.subject_sha256.clone(),
        expectation_bound: true,
    })
}

fn valid_provenance_expectation(value: &ProvenanceExpectation) -> bool {
    digest(&value.context_id)
        && digest(&value.candidate_id)
        && digest(&value.subject_sha256)
        && !value.subject_name.is_empty()
        && !value.builder_id.is_empty()
        && !value.predicate_type.is_empty()
        && value.materials.len() <= 4096
        && value
            .materials
            .iter()
            .all(|(uri, digest_value)| !uri.is_empty() && uri.len() <= 512 && digest(digest_value))
        && value.materials.keys().collect::<BTreeSet<_>>().len() == value.materials.len()
}
