use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[cfg(test)]
use crate::capture::{ArtifactDisposition, ArtifactResolver, CapturedArtifact};
#[cfg(not(test))]
use crate::capture::{ArtifactDisposition, ArtifactResolver, CapturedArtifact};

use super::model::{
    EvidenceBinding, ReceiptState, ReuseExpectation, RunOutcome, receipt_error, valid_result_map,
};
use super::witness::ExecutedWork;
use crate::routine_work::RoutineError;
use crate::routine_work::digest::{canonical, sha256, valid};

const MAX_RECEIPT_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReceiptWire {
    pub(super) schema_version: String,
    pub(super) binding: EvidenceBinding,
    pub(super) state: ReceiptState,
    pub(super) outcome: RunOutcome,
    pub(super) behavior_observed: bool,
    pub(super) behavior_sha256: String,
    pub(super) output_digests: BTreeMap<String, String>,
    pub(super) result_artifact_sha256: String,
    pub(super) producer_capture_run_sha256: String,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ReuseReceipt {
    pub(super) wire: ReceiptWire,
    receipt_artifact_sha256: String,
}

impl ReuseReceipt {
    pub fn from_captured(artifact: &CapturedArtifact) -> Result<Self, RoutineError> {
        if artifact.disposition() != ArtifactDisposition::Public
            || artifact.byte_length() as usize > MAX_RECEIPT_BYTES
        {
            return Err(receipt_error("receipt-artifact-authority-invalid"));
        }
        let reference = artifact.artifact_ref();
        let bytes = artifact
            .resolve(&reference)
            .map_err(|_| receipt_error("receipt-artifact-resolution-failed"))?;
        let wire = parse_wire(&bytes)?;
        if artifact.context_id() != wire.binding.context_id
            || artifact.candidate_id() != wire.binding.candidate_id
        {
            return Err(receipt_error("receipt-artifact-binding-mismatch"));
        }
        Ok(Self {
            wire,
            receipt_artifact_sha256: reference.sha256().to_owned(),
        })
    }

    pub fn started_bytes(expectation: &ReuseExpectation) -> Result<Vec<u8>, RoutineError> {
        canonical(&wire(
            expectation,
            ReceiptState::Started,
            RunOutcome::Interrupted,
            false,
            sha256(b"no-completed-behavior"),
            BTreeMap::new(),
            sha256(b"no-result-artifact"),
            sha256(b"no-producer-capture"),
        ))
    }

    pub fn to_canonical_json(&self) -> Result<Vec<u8>, RoutineError> {
        canonical(&self.wire)
    }

    pub fn artifact_sha256(&self) -> &str {
        &self.receipt_artifact_sha256
    }
}

pub(super) fn completed_bytes(
    expectation: &ReuseExpectation,
    work: &ExecutedWork,
) -> Result<Vec<u8>, RoutineError> {
    if work.facts.binding != expectation.binding {
        return Err(receipt_error("executed-work-expectation-mismatch"));
    }
    canonical(&wire(
        expectation,
        ReceiptState::Complete,
        work.facts.outcome,
        work.facts.behavior_observed,
        work.facts.behavior_sha256.clone(),
        work.facts.output_digests.clone(),
        work.facts.result_artifact_sha256.clone(),
        work.capture_run_sha256.clone(),
    ))
}

fn wire(
    expectation: &ReuseExpectation,
    state: ReceiptState,
    outcome: RunOutcome,
    behavior_observed: bool,
    behavior_sha256: String,
    output_digests: BTreeMap<String, String>,
    result_artifact_sha256: String,
    producer_capture_run_sha256: String,
) -> ReceiptWire {
    ReceiptWire {
        schema_version: "VerifiedReuse-v2".to_owned(),
        binding: expectation.binding.clone(),
        state,
        outcome,
        behavior_observed,
        behavior_sha256,
        output_digests,
        result_artifact_sha256,
        producer_capture_run_sha256,
    }
}

fn parse_wire(bytes: &[u8]) -> Result<ReceiptWire, RoutineError> {
    if bytes.len() > MAX_RECEIPT_BYTES {
        return Err(receipt_error("receipt-size-limit-exceeded"));
    }
    let wire: ReceiptWire =
        serde_json::from_slice(bytes).map_err(|_| receipt_error("receipt-json-invalid"))?;
    if canonical(&wire)? != bytes || !valid_wire(&wire) {
        return Err(receipt_error("receipt-fields-or-canonical-form-invalid"));
    }
    Ok(wire)
}

fn valid_wire(wire: &ReceiptWire) -> bool {
    wire.schema_version == "VerifiedReuse-v2"
        && wire.binding.valid()
        && valid(&wire.behavior_sha256)
        && valid(&wire.result_artifact_sha256)
        && valid(&wire.producer_capture_run_sha256)
        && valid_result_map(&wire.output_digests)
}
